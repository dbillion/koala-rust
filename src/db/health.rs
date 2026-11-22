use super::{Database, HealthInfo};
use anyhow::Result;
use sqlx::Row;

impl Database {
    // ===== HEALTH =====
    pub async fn health(&self) -> Result<HealthInfo> {
        let tables: i64 = sqlx::query("SELECT COUNT(*) as c FROM sqlite_master WHERE type='table'")
            .fetch_one(&self.pool)
            .await?
            .get("c");

        let pages: i64 = sqlx::query("PRAGMA page_count")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        let journal_mode: String = sqlx::query("PRAGMA journal_mode")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        let journal = journal_mode != "off";

        Ok(HealthInfo {
            tables,
            pages,
            journal,
            cache_used: 0,
            usage_tracked: 0,
            pending_flush: false,
        })
    }
}
