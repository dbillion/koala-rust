use super::Database;
use anyhow::Result;

impl Database {
    // ===== RANKING =====
    pub async fn feedback(&self, name: &str, signal: &str, kind: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO rank_signals (name, signal_type, value) VALUES (?, ?, 1.0)
             ON CONFLICT(name) DO UPDATE SET signal_type = ?, value = rank_signals.value + 1.0, updated_at = CURRENT_TIMESTAMP"
        )
        .bind(name)
        .bind(format!("{}:{}", kind, signal))
        .bind(format!("{}:{}", kind, signal))
        .execute(&mut *tx).await?;
        super::audit::append_tx(
            &mut tx,
            &super::audit::actor(),
            "feedback.record",
            name,
            serde_json::json!({"kind": kind, "signal": signal}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
