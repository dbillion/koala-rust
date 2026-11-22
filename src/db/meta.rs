use super::Database;
use anyhow::Result;
use sqlx::Row;

impl Database {
    pub async fn meta_get(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT value FROM brain_meta WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get("value")))
    }

    pub async fn meta_set(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO brain_meta(key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
