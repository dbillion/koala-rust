use super::Database;
use anyhow::Result;

impl Database {
    /// Reclaim pages released by a secure purge.
    pub async fn incremental_vacuum(&self) -> Result<()> {
        sqlx::query("PRAGMA incremental_vacuum")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
