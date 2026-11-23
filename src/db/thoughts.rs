use super::Database;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;

impl Database {
    // ===== THOUGHTS =====
    pub async fn add_thought(&self, step: i64, thought: &str) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("INSERT INTO thoughts (step, thought) VALUES (?, ?) RETURNING rowid")
            .bind(step)
            .bind(thought)
            .fetch_one(&mut *tx)
            .await?;
        let rowid: i64 = row.get("rowid");
        super::audit::append_tx(
            &mut tx,
            &super::audit::actor(),
            "thought.create",
            &format!("thought:{rowid}"),
            serde_json::json!({"step": step, "content_bytes": thought.len()}),
        )
        .await?;
        tx.commit().await?;
        Ok(rowid)
    }

    pub async fn get_thoughts(&self, limit: i64) -> Result<Vec<(i64, i64, String, DateTime<Utc>)>> {
        let rows = sqlx::query(
            "SELECT rowid, step, thought, created_at FROM thoughts ORDER BY step ASC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get("rowid"),
                    r.get("step"),
                    r.get("thought"),
                    r.get("created_at"),
                )
            })
            .collect())
    }

    pub async fn clear_thoughts(&self) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query("DELETE FROM thoughts")
            .execute(&mut *tx)
            .await?;
        let deleted = result.rows_affected() as i64;
        if deleted > 0 {
            super::audit::append_tx(
                &mut tx,
                &super::audit::actor(),
                "thought.clear",
                "thoughts",
                serde_json::json!({"deleted": deleted}),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(deleted)
    }

    // ===== CONCLUSIONS =====
    pub async fn add_conclusion(&self, summary: &str) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("INSERT INTO conclusions (summary) VALUES (?) RETURNING rowid")
            .bind(summary)
            .fetch_one(&mut *tx)
            .await?;
        let rowid: i64 = row.get("rowid");
        super::audit::append_tx(
            &mut tx,
            &super::audit::actor(),
            "conclusion.create",
            &format!("conclusion:{rowid}"),
            serde_json::json!({"summary_bytes": summary.len()}),
        )
        .await?;
        tx.commit().await?;
        Ok(rowid)
    }

    pub async fn get_conclusions(&self) -> Result<Vec<(i64, String, DateTime<Utc>)>> {
        let rows = sqlx::query(
            "SELECT rowid, summary, created_at FROM conclusions ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get("rowid"), r.get("summary"), r.get("created_at")))
            .collect())
    }
}
