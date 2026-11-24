//! Vector Search Module (v0.2)
//!
//! Implements semantic search using sqlite-vec extension.
//! Exact (brute-force) KNN over sqlite-vec vec0 tables for facts and entities.
//! There is no HNSW index; the hnsw_* config fields are unused.

use anyhow::Result;
use sqlx::{AssertSqlSafe, QueryBuilder, Row, Sqlite, SqlitePool};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Default vector dimension (potion-base-8M). The live value comes from
/// `VectorEngine::dimension()` and env KOALA_EMBED_DIM.
pub const VECTOR_DIM: usize = crate::embeddings::EMBEDDING_DIM;

/// Vector search configuration
#[derive(Debug, Clone)]
pub struct VectorConfig {
    pub dimension: usize,
    pub hnsw_m: usize,
    pub hnsw_ef_construction: usize,
    pub hnsw_ef_search: usize,
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            dimension: crate::embeddings::configured_dim(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
            hnsw_ef_search: 64,
        }
    }
}

/// Vector search result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorSearchResult {
    pub id: i64,
    pub score: f32,
    pub metadata: serde_json::Value,
}

/// Vector search engine
pub struct VectorEngine {
    pool: SqlitePool,
    config: VectorConfig,
    initialized: Arc<RwLock<bool>>,
    rebuilt: bool,
}

impl VectorEngine {
    pub async fn new(pool: SqlitePool, config: VectorConfig) -> Result<Self> {
        let mut engine = Self {
            pool,
            config,
            initialized: Arc::new(RwLock::new(false)),
            rebuilt: false,
        };
        engine.rebuilt = engine.migrate_dimension().await?;
        engine.init().await?;
        Ok(engine)
    }

    /// True when an old vector table with a different dimension was dropped
    /// at startup. Caller should re-embed facts (see Koala::reindex_facts).
    pub fn was_rebuilt(&self) -> bool {
        self.rebuilt
    }

    /// Dimension the engine is configured for.
    pub fn dimension(&self) -> usize {
        self.config.dimension
    }

    /// If an existing vec0 table was created with another dimension, drop it
    /// so it is recreated with the current one. Vectors are re-derivable from
    /// fact text, so nothing authoritative is lost.
    async fn migrate_dimension(&self) -> Result<bool> {
        let row = sqlx::query("SELECT sql FROM sqlite_master WHERE name = 'fact_embeddings'")
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(false);
        };
        let sql: String = row.get("sql");
        let stored = sql
            .to_uppercase()
            .split("FLOAT[")
            .nth(1)
            .and_then(|s| s.split(']').next().map(|d| d.trim().to_string()))
            .and_then(|d| d.parse::<usize>().ok());
        match stored {
            Some(d) if d != self.config.dimension => {
                tracing::warn!(
                    "Vector tables were {} dims, model needs {}. Rebuilding (entity vectors are dropped, fact vectors get re-embedded).",
                    d,
                    self.config.dimension
                );
                // Persist the marker first: if we crash after the drop, the next
                // startup still knows the vectors must be rebuilt.
                sqlx::query(
                    "INSERT OR REPLACE INTO brain_meta(key, value) VALUES ('reindex_pending', '1')",
                )
                .execute(&self.pool)
                .await?;
                sqlx::query("DROP TABLE IF EXISTS fact_embeddings")
                    .execute(&self.pool)
                    .await?;
                sqlx::query("DROP TABLE IF EXISTS entity_embeddings")
                    .execute(&self.pool)
                    .await?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Initialize sqlite-vec extension and create virtual tables
    async fn init(&self) -> Result<()> {
        let mut initialized = self.initialized.write().await;
        if *initialized {
            return Ok(());
        }

        // Extension is now loaded on every connection via Database pool
        // Just create virtual tables
        self.create_tables().await?;

        *initialized = true;
        Ok(())
    }

    #[allow(dead_code)]
    async fn load_extension(&self) -> Result<()> {
        // No-op: extension loaded at pool level via SqliteConnectOptions
        Ok(())
    }

    async fn create_tables(&self) -> Result<()> {
        let dim = self.config.dimension;
        for table in ["fact_embeddings", "entity_embeddings"] {
            // A vec0 table created under the previous embedding default (384)
            // keeps its original width because of CREATE VIRTUAL TABLE ... IF
            // NOT EXISTS, so the stored vectors would never match the current
            // 256-dim embeddings. Rebuild the table (and its HNSW index) when
            // the stored width differs; legacy vectors are re-embedded by the
            // normal write / reindex paths as new data is written.
            if self.stale_vec0_table(table, dim).await? {
                // SAFETY OF SQL STRING: `table` comes only from the fixed
                // fact_embeddings/entity_embeddings allowlist above.
                sqlx::query(AssertSqlSafe(format!("DROP TABLE IF NOT EXISTS {table}")))
                    .execute(&self.pool)
                    .await?;
            }
            // SAFETY OF SQL STRING: `table` is allowlisted above and `dim` is
            // an integer, so neither value can inject SQL syntax.
            sqlx::query(AssertSqlSafe(format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS {table} USING vec0(embedding FLOAT[{dim}])"
            )))
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Returns `true` when `{table}` already exists with a vec0 column width
    /// that differs from `dim`. The stored CREATE VIRTUAL TABLE SQL is
    /// consulted so a legacy 384-dimensional table is detected on upgrades.
    async fn stale_vec0_table(&self, table: &str, dim: usize) -> Result<bool> {
        let sql = sqlx::query_scalar::<_, String>(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?",
        )
        .bind(table)
        .fetch_optional(&self.pool)
        .await?;
        let Some(sql) = sql else {
            return Ok(false);
        };
        let stored = Self::vec0_dim(&sql).unwrap_or(0);
        Ok(stored != dim)
    }

    /// Extracts the column width from a stored vec0 CREATE statement, e.g.
    /// "vec0(embedding FLOAT[384])" -> 384.
    fn vec0_dim(sql: &str) -> Option<usize> {
        let args = sql.split_once('(')?.1;
        args.split("FLOAT[").nth(1)?.split(']').next()?.parse().ok()
    }

    /// Upsert a fact embedding
    pub async fn upsert_fact_embedding(&self, fact_id: i64, embedding: &[f32]) -> Result<()> {
        if embedding.len() != self.config.dimension {
            anyhow::bail!(
                "Embedding dimension mismatch: expected {}, got {}",
                self.config.dimension,
                embedding.len()
            );
        }

        // Convert to JSON array for sqlite-vec
        let embedding_json = serde_json::to_string(embedding)?;

        sqlx::query("INSERT OR REPLACE INTO fact_embeddings(rowid, embedding) VALUES (?, ?)")
            .bind(fact_id)
            .bind(embedding_json)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Upsert an entity embedding
    pub async fn upsert_entity_embedding(&self, entity_id: i64, embedding: &[f32]) -> Result<()> {
        if embedding.len() != self.config.dimension {
            anyhow::bail!("Embedding dimension mismatch");
        }

        let embedding_json = serde_json::to_string(embedding)?;

        sqlx::query("INSERT OR REPLACE INTO entity_embeddings(rowid, embedding) VALUES (?, ?)")
            .bind(entity_id)
            .bind(embedding_json)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Search fact embeddings by vector similarity. `filter`, when present,
    /// is a literal keyword matched against current fact names and text.
    pub async fn search_facts(
        &self,
        query_embedding: &[f32],
        k: usize,
        filter: Option<&str>,
    ) -> Result<Vec<VectorSearchResult>> {
        if query_embedding.len() != self.config.dimension {
            anyhow::bail!("Query embedding dimension mismatch");
        }

        let query_json = serde_json::to_string(query_embedding)?;
        let _ef_search = self.config.hnsw_ef_search;

        // `filter` is a literal keyword, not a SQL expression. Keep it bound
        // so callers cannot inject predicates or alter the query structure.
        let keyword = filter.map(str::trim).filter(|value| !value.is_empty());
        let mut query = QueryBuilder::<Sqlite>::new("SELECT rowid as id, distance as score, ");
        query
            .push_bind(keyword.unwrap_or_default())
            .push(" as metadata FROM fact_embeddings WHERE embedding MATCH ")
            .push_bind(query_json);
        if let Some(keyword) = keyword {
            query
                .push(" AND rowid IN (SELECT rowid FROM facts WHERE is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP) AND (instr(lower(name), lower(")
                .push_bind(keyword)
                .push(")) > 0 OR instr(lower(text), lower(")
                .push_bind(keyword)
                .push(")) > 0))");
        }
        query
            .push(" ORDER BY distance ASC LIMIT ")
            .push_bind(k as i64);
        let rows = query.build().fetch_all(&self.pool).await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            results.push(VectorSearchResult {
                id: row.get("id"),
                score: row.get("score"),
                metadata: row.get("metadata"),
            });
        }

        Ok(results)
    }

    /// Search entity embeddings by vector similarity
    pub async fn search_entities(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<VectorSearchResult>> {
        if query_embedding.len() != self.config.dimension {
            anyhow::bail!("Query embedding dimension mismatch");
        }

        let query_json = serde_json::to_string(query_embedding)?;

        let rows = sqlx::query(
            "SELECT rowid as id, distance as score, '{}' as metadata
             FROM entity_embeddings 
             WHERE embedding MATCH ? 
             ORDER BY distance ASC 
             LIMIT ?",
        )
        .bind(query_json)
        .bind(k as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            results.push(VectorSearchResult {
                id: row.get("id"),
                score: row.get("score"),
                metadata: row.get("metadata"),
            });
        }

        Ok(results)
    }

    /// Delete fact embedding
    pub async fn delete_fact_embedding(&self, fact_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM fact_embeddings WHERE rowid = ?")
            .bind(fact_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete entity embedding
    pub async fn delete_entity_embedding(&self, entity_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM entity_embeddings WHERE rowid = ?")
            .bind(entity_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Hybrid search: combine vector similarity with graph traversal
    pub async fn hybrid_search(
        &self,
        query_embedding: &[f32],
        k: usize,
        _graph_depth: usize,
    ) -> Result<Vec<VectorSearchResult>> {
        // First: vector search
        let vector_results = self.search_facts(query_embedding, k * 2, None).await?;

        // Then: expand via graph relations (if entities are linked)
        // This would join with entity_relations table
        // For now, return vector results
        Ok(vector_results.into_iter().take(k).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn test_vector_engine_rejects_wrong_embedding_dimensions() {
        // This assertion exercises validation before any database query, so
        // use a plain in-memory SQLite pool without loading sqlite-vec.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let engine = VectorEngine {
            pool,
            config: VectorConfig {
                dimension: VECTOR_DIM,
                ..VectorConfig::default()
            },
            initialized: Arc::new(RwLock::new(false)),
            rebuilt: false,
        };
        let wrong_dimension = vec![0.1f32; VECTOR_DIM + 1];

        let error = engine
            .upsert_fact_embedding(1, &wrong_dimension)
            .await
            .expect_err("an embedding with the wrong dimension must be rejected");

        assert!(error.to_string().contains("Embedding dimension mismatch"));
    }
}
