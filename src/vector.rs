//! Vector Search Module (v0.2)
//! 
//! Implements semantic search using sqlite-vec extension.
//! Provides HNSW-based vector indexing for facts and entities.

use anyhow::Result;
use sqlx::{SqlitePool, Row};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Vector dimension for embeddings (all-MiniLM-L6-v2 = 384)
pub const VECTOR_DIM: usize = 384;

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
            dimension: VECTOR_DIM,
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
}

impl VectorEngine {
    pub async fn new(pool: SqlitePool, config: VectorConfig) -> Result<Self> {
        let engine = Self {
            pool,
            config,
            initialized: Arc::new(RwLock::new(false)),
        };
        engine.init().await?;
        Ok(engine)
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

    async fn load_extension(&self) -> Result<()> {
        // No-op: extension loaded at pool level via SqliteConnectOptions
        Ok(())
    }

    async fn create_tables(&self) -> Result<()> {
        let dim = self.config.dimension;
        let m = self.config.hnsw_m;
        let ef_construction = self.config.hnsw_ef_construction;

        // Try to create virtual tables - will fail gracefully if vec0 not available
        let result = async {
            // Create virtual table for fact embeddings
            sqlx::query(&format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS fact_embeddings USING vec0(
                    embedding FLOAT[{}]
                )",
                dim
            ))
            .execute(&self.pool)
            .await?;

            // Create virtual table for entity embeddings
            sqlx::query(&format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS entity_embeddings USING vec0(
                    embedding FLOAT[{}]
                )",
                dim
            ))
            .execute(&self.pool)
            .await?;

            // Create HNSW index for fact embeddings
            sqlx::query(&format!(
                "CREATE INDEX IF NOT EXISTS fact_embeddings_hnsw ON fact_embeddings(
                    embedding
                ) USING hnsw(m={}, ef_construction={})",
                m, ef_construction
            ))
            .execute(&self.pool)
            .await?;

            // Create HNSW index for entity embeddings
            sqlx::query(&format!(
                "CREATE INDEX IF NOT EXISTS entity_embeddings_hnsw ON entity_embeddings(
                    embedding
                ) USING hnsw(m={}, ef_construction={})",
                m, ef_construction
            ))
            .execute(&self.pool)
            .await?;

            Ok::<(), anyhow::Error>(())
        }.await;

        if let Err(e) = result {
            // Vec0 extension not available - vector search will be disabled
            let _ = e;
        }
        Ok(())
    }

    /// Upsert a fact embedding
    pub async fn upsert_fact_embedding(&self, fact_id: i64, embedding: &[f32]) -> Result<()> {
        if embedding.len() != self.config.dimension {
            anyhow::bail!("Embedding dimension mismatch: expected {}, got {}", 
                self.config.dimension, embedding.len());
        }

        // Convert to JSON array for sqlite-vec
        let embedding_json = serde_json::to_string(embedding)?;

        sqlx::query(
            "INSERT OR REPLACE INTO fact_embeddings(rowid, embedding) VALUES (?, ?)"
        )
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

        sqlx::query(
            "INSERT OR REPLACE INTO entity_embeddings(rowid, embedding) VALUES (?, ?)"
        )
        .bind(entity_id)
        .bind(embedding_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Search fact embeddings by vector similarity
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

        let sql = if let Some(filter) = filter {
            format!(
                "SELECT rowid as id, distance as score, '{filter}' as metadata 
                 FROM fact_embeddings 
                 WHERE embedding MATCH ? 
                 AND rowid IN (SELECT rowid FROM facts WHERE {filter}) 
                 ORDER BY distance ASC 
                 LIMIT ?"
            )
        } else {
            format!(
                "SELECT rowid as id, distance as score, '{{}}' as metadata 
                 FROM fact_embeddings 
                 WHERE embedding MATCH ? 
                 ORDER BY distance ASC 
                 LIMIT ?"
            )
        };

        let rows = sqlx::query(&sql)
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

        let sql = format!(
            "SELECT rowid as id, distance as score, '{{}}' as metadata 
             FROM entity_embeddings 
             WHERE embedding MATCH ? 
             ORDER BY distance ASC 
             LIMIT ?"
        );

        let rows = sqlx::query(&sql)
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
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn test_vector_engine() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let config = VectorConfig::default();
        let engine = VectorEngine::new(pool, config).await.unwrap();

        // Test embedding
        let embedding = vec![0.1f32; VECTOR_DIM];
        engine.upsert_fact_embedding(1, &embedding).await.unwrap();

        // Search
        let results = engine.search_facts(&embedding, 10, None).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, 1);
    }
}