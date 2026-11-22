use super::Database;
use anyhow::Result;
use sqlx::Row;

impl Database {
    // ===== FACTS =====
    pub async fn write_fact(&self, name: &str, text: &str) -> Result<i64> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE facts SET is_current = 0 WHERE name = ? AND is_current = 1")
            .bind(name)
            .execute(&mut *tx)
            .await?;

        let row = sqlx::query(
            "INSERT INTO facts (name, text, is_current) VALUES (?, ?, 1) RETURNING rowid",
        )
        .bind(name)
        .bind(text)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row.get("rowid"))
    }

    pub async fn read_fact(&self, name: &str) -> Result<Option<(i64, String)>> {
        let row = sqlx::query("SELECT rowid, text FROM facts WHERE name = ? AND is_current = 1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| (r.get("rowid"), r.get("text"))))
    }

    /// Fetch a fact by rowid. Returns (name, text, is_current).
    pub async fn read_fact_by_rowid(&self, rowid: i64) -> Result<Option<(String, String, bool)>> {
        let row = sqlx::query("SELECT name, text, is_current FROM facts WHERE rowid = ?")
            .bind(rowid)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| (r.get("name"), r.get("text"), r.get::<bool, _>("is_current"))))
    }

    /// All current facts as (rowid, name, text).
    pub async fn current_facts(&self) -> Result<Vec<(i64, String, String)>> {
        let rows = sqlx::query("SELECT rowid, name, text FROM facts WHERE is_current = 1")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get("rowid"), r.get("name"), r.get("text")))
            .collect())
    }

    pub async fn list_facts(&self) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT name FROM facts WHERE is_current = 1 ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get("name")).collect())
    }

    pub async fn delete_fact(&self, name: &str) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let deleted = sqlx::query("DELETE FROM facts WHERE name = ? AND is_current = 1")
            .bind(name)
            .execute(&mut *tx)
            .await?
            .rows_affected();

        if deleted > 0 {
            sqlx::query("INSERT OR IGNORE INTO facts_tombstone (name) VALUES (?)")
                .bind(name)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(deleted > 0)
    }
}
