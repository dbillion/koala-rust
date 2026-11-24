use super::Database;
use anyhow::Result;
use sqlx::{QueryBuilder, Row, Sqlite};

impl Database {
    /// Rowids of current facts whose name or text contains any of the tokens.
    /// Independent of the vector index, so exact names outside the vector
    /// candidate pool are still found.
    pub async fn keyword_fact_candidates(&self, tokens: &[String], limit: i64) -> Result<Vec<i64>> {
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT rowid FROM facts WHERE is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP) AND (",
        );
        for (index, token) in tokens.iter().enumerate() {
            if index > 0 {
                query.push(" OR ");
            }
            query
                .push("(instr(lower(replace(name, '_', ' ')), lower(")
                .push_bind(token.as_str())
                .push(")) > 0 OR instr(lower(text), lower(")
                .push_bind(token.as_str())
                .push(")) > 0)");
        }
        query.push(") LIMIT ").push_bind(limit);
        let rows = query.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| r.get("rowid")).collect())
    }

    /// Entities as (rowid, text to embed): name, type and observations.
    pub async fn entity_texts(&self) -> Result<Vec<(i64, String)>> {
        let rows = sqlx::query(
            "SELECT e.rowid AS rowid, e.name AS name, e.type AS type, \
             group_concat(o.content, '; ') AS obs \
             FROM entities e LEFT JOIN observations o ON o.entity_name = e.name \
             GROUP BY e.rowid",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let name: String = r.get("name");
                let ty: Option<String> = r.get("type");
                let obs: Option<String> = r.get("obs");
                let text = format!(
                    "{} ({}): {}",
                    name,
                    ty.unwrap_or_default(),
                    obs.unwrap_or_default()
                );
                (r.get("rowid"), text)
            })
            .collect())
    }

    /// Rowids of the current version(s) of a fact name.
    pub async fn current_fact_rowids(&self, name: &str) -> Result<Vec<i64>> {
        let rows = sqlx::query("SELECT rowid FROM facts WHERE name = ? AND is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP)")
            .bind(name)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get("rowid")).collect())
    }
}

impl Database {
    /// Rowids for every retained version of a fact name. Used by the irreversible
    /// purge path so historical vectors are removed along with historical text.
    pub async fn fact_rowids(&self, name: &str) -> Result<Vec<i64>> {
        let rows = sqlx::query("SELECT rowid FROM facts WHERE name = ?")
            .bind(name)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get("rowid")).collect())
    }
}
