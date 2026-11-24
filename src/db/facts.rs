use super::Database;
use anyhow::Result;
use chrono::{Duration, Utc};
use sqlx::Row;

impl Database {
    // ===== FACTS =====
    pub async fn write_fact(&self, name: &str, text: &str) -> Result<i64> {
        self.write_fact_with_ttl(name, text, None).await
    }

    /// Write a new fact version with an optional TTL in seconds.
    pub async fn write_fact_with_ttl(
        &self,
        name: &str,
        text: &str,
        ttl_seconds: Option<i64>,
    ) -> Result<i64> {
        if ttl_seconds.is_some_and(|ttl| ttl <= 0) {
            anyhow::bail!("ttl_seconds must be greater than zero");
        }
        let expires_at = ttl_seconds.map(|ttl| Utc::now() + Duration::seconds(ttl));
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM facts_tombstone WHERE name = ?")
            .bind(name)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE facts SET is_current = 0 WHERE name = ? AND is_current = 1")
            .bind(name)
            .execute(&mut *tx)
            .await?;

        let row = sqlx::query(
            "INSERT INTO facts (name, text, is_current, expires_at) VALUES (?, ?, 1, ?) RETURNING rowid",
        )
        .bind(name)
        .bind(text)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;

        let rowid: i64 = row.get("rowid");
        super::audit::append_tx(
            &mut tx,
            &super::audit::actor(),
            "fact.write",
            name,
            serde_json::json!({"rowid": rowid, "ttl_seconds": ttl_seconds}),
        )
        .await?;
        tx.commit().await?;
        Ok(rowid)
    }

    pub async fn read_fact(&self, name: &str) -> Result<Option<(i64, String)>> {
        let row = sqlx::query("SELECT rowid, text FROM facts WHERE name = ? AND is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP)")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| (r.get("rowid"), r.get("text"))))
    }

    /// Fetch a fact by rowid. Returns (name, text, is_current).
    pub async fn read_fact_by_rowid(&self, rowid: i64) -> Result<Option<(String, String, bool)>> {
        let row = sqlx::query("SELECT name, text, (is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP)) AS is_current FROM facts WHERE rowid = ?")
            .bind(rowid)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| (r.get("name"), r.get("text"), r.get::<bool, _>("is_current"))))
    }

    /// All current facts as (rowid, name, text).
    pub async fn current_facts(&self) -> Result<Vec<(i64, String, String)>> {
        let rows = sqlx::query("SELECT rowid, name, text FROM facts WHERE is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP)")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get("rowid"), r.get("name"), r.get("text")))
            .collect())
    }

    pub async fn list_facts(&self) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT name FROM facts WHERE is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP) ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get("name")).collect())
    }
}

mod retention;
mod vacuum;

#[cfg(test)]
mod tests;
