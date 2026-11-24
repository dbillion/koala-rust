use anyhow::Result;
use sqlx::{Row, Sqlite, Transaction};

/// Add audit metadata to text-free fact tombstones. The name and timestamp are
/// sufficient for old databases; new deletions also record local actor/reason.
pub(super) async fn run(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
    let columns = sqlx::query("PRAGMA table_info(facts_tombstone)")
        .fetch_all(&mut **tx)
        .await?;
    let has_actor = columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "actor");
    let has_reason = columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "reason");
    if !has_actor {
        sqlx::query("ALTER TABLE facts_tombstone ADD COLUMN actor TEXT NOT NULL DEFAULT 'local'")
            .execute(&mut **tx)
            .await?;
    }
    if !has_reason {
        sqlx::query(
            "ALTER TABLE facts_tombstone ADD COLUMN reason TEXT NOT NULL DEFAULT 'user_requested'",
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
