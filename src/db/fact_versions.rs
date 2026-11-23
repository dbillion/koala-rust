use super::Database;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;

impl Database {
    /// Read the newest retained version of a fact that existed at `as_of`.
    pub async fn read_fact_as_of(
        &self,
        name: &str,
        as_of: &str,
    ) -> Result<Option<(i64, String, DateTime<Utc>)>> {
        let timestamp: Option<String> = sqlx::query_scalar("SELECT datetime(?)")
            .bind(as_of)
            .fetch_one(&self.pool)
            .await?;
        let timestamp = timestamp.ok_or_else(|| anyhow::anyhow!("invalid timestamp: {}", as_of))?;
        let row = sqlx::query(
            "SELECT rowid, text, created_at FROM facts WHERE name = ? AND datetime(created_at) <= datetime(?) AND (expires_at IS NULL OR datetime(expires_at) > datetime(?)) ORDER BY datetime(created_at) DESC, rowid DESC LIMIT 1",
        )
        .bind(name)
        .bind(&timestamp)
        .bind(&timestamp)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| (r.get("rowid"), r.get("text"), r.get("created_at"))))
    }

    /// One latest version per fact name as it existed at a point in time.
    pub async fn facts_as_of(&self, as_of: &str) -> Result<Vec<(i64, String, String)>> {
        let timestamp: Option<String> = sqlx::query_scalar("SELECT datetime(?)")
            .bind(as_of)
            .fetch_one(&self.pool)
            .await?;
        let timestamp = timestamp.ok_or_else(|| anyhow::anyhow!("invalid timestamp: {}", as_of))?;
        let rows = sqlx::query(
            "SELECT f.rowid, f.name, f.text FROM facts f WHERE datetime(f.created_at) <= datetime(?) AND (f.expires_at IS NULL OR datetime(f.expires_at) > datetime(?)) AND f.rowid = (SELECT prior.rowid FROM facts prior WHERE prior.name = f.name AND datetime(prior.created_at) <= datetime(?) AND (prior.expires_at IS NULL OR datetime(prior.expires_at) > datetime(?)) ORDER BY datetime(prior.created_at) DESC, prior.rowid DESC LIMIT 1) ORDER BY f.name",
        )
        .bind(&timestamp)
        .bind(&timestamp)
        .bind(&timestamp)
        .bind(&timestamp)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get("rowid"), r.get("name"), r.get("text")))
            .collect())
    }

    pub async fn read_fact_versions(
        &self,
        name: &str,
        limit: i64,
    ) -> Result<Vec<(i64, String, DateTime<Utc>, bool)>> {
        let rows = sqlx::query("SELECT rowid, text, created_at, (is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP)) AS is_current FROM facts WHERE name = ? ORDER BY created_at DESC LIMIT ?")
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

#[cfg(test)]
mod tests;
