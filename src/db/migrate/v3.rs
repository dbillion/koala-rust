use anyhow::Result;
use sqlx::{Row, Sqlite, Transaction};

/// Add optional per-fact expiry metadata and an index for retention sweeps.
pub(super) async fn run(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
    let columns = sqlx::query("PRAGMA table_info(facts)")
        .fetch_all(&mut **tx)
        .await?;
    let has_expires_at = columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "expires_at");
    if !has_expires_at {
        sqlx::query("ALTER TABLE facts ADD COLUMN expires_at DATETIME")
            .execute(&mut **tx)
            .await?;
    }
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_facts_expires_at ON facts(expires_at)")
        .execute(&mut **tx)
        .await?;
    Ok(())
}
