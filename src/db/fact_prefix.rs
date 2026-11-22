use super::Database;
use anyhow::Result;
use sqlx::Row;

impl Database {
    /// Rowids of current facts whose name starts with the prefix.
    pub async fn current_fact_rowids_prefix(&self, prefix: &str) -> Result<Vec<i64>> {
        let rows = sqlx::query("SELECT rowid FROM facts WHERE name LIKE ? AND is_current = 1")
            .bind(format!("{}%", prefix))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get("rowid")).collect())
    }

    pub async fn delete_fact_prefix(&self, prefix: &str) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        let names: Vec<String> =
            sqlx::query("SELECT name FROM facts WHERE name LIKE ? AND is_current = 1")
                .bind(format!("{}%", prefix))
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .map(|r| r.get("name"))
                .collect();

        let count = names.len() as i64;
        for name in names {
            sqlx::query("UPDATE facts SET is_current = 0 WHERE name = ? AND is_current = 1")
                .bind(&name)
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT OR IGNORE INTO facts_tombstone (name) VALUES (?)")
                .bind(&name)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(count)
    }
}
