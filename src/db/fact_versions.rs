use super::Database;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;

impl Database {
    pub async fn read_fact_versions(
        &self,
        name: &str,
        limit: i64,
    ) -> Result<Vec<(i64, String, DateTime<Utc>, bool)>> {
        let rows = sqlx::query("SELECT rowid, text, created_at, is_current FROM facts WHERE name = ? ORDER BY created_at DESC LIMIT ?")
            .bind(name)
            .bind(limit)
            .fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get("rowid"),
                    r.get("text"),
                    r.get("created_at"),
                    r.get("is_current"),
                )
            })
            .collect())
    }
}
