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
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT OR REPLACE INTO brain_meta(key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&mut *tx)
            .await?;
        super::audit::append_tx(
            &mut tx,
            &super::audit::actor(),
            "meta.set",
            key,
            serde_json::json!({"value_bytes": value.len()}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
