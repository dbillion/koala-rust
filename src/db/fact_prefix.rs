use super::Database;
use anyhow::Result;
use sqlx::Row;

impl Database {
    /// Rowids of current facts whose name starts with the prefix.
    pub async fn current_fact_rowids_prefix(&self, prefix: &str) -> Result<Vec<i64>> {
        let rows = sqlx::query("SELECT rowid FROM facts WHERE name LIKE ? AND is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP)")
            .bind(format!("{}%", prefix))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get("rowid")).collect())
    }

    /// Rowids for all retained versions of facts whose names start with prefix.
    pub async fn fact_rowids_prefix(&self, prefix: &str) -> Result<Vec<i64>> {
        let rows = sqlx::query("SELECT rowid FROM facts WHERE name LIKE ?")
            .bind(format!("{}%", prefix))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get("rowid")).collect())
    }

    pub async fn delete_fact_prefix(&self, prefix: &str) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        let names: Vec<String> = sqlx::query("SELECT DISTINCT name FROM facts WHERE name LIKE ?")
            .bind(format!("{}%", prefix))
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|r| r.get("name"))
            .collect();

        let count = names.len() as i64;
        for name in names {
            sqlx::query("DELETE FROM facts WHERE name = ?")
                .bind(&name)
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT OR REPLACE INTO facts_tombstone (name, actor, reason) VALUES (?, 'local', 'prefix_delete')")
                .bind(&name).execute(&mut *tx).await?;
            super::audit::append_tx(
                &mut tx,
                &super::audit::actor(),
                "fact.delete",
                &name,
                serde_json::json!({"reason": "prefix_delete"}),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(count)
    }
}
