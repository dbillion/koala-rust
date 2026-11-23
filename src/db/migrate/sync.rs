use anyhow::Result;
use sqlx::{Sqlite, Transaction};

// v0.3: Sync metadata tables.
pub(super) async fn run(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS sync_peers (
                peer_id TEXT PRIMARY KEY,
                last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
                doc_type TEXT NOT NULL,
                heads_json TEXT NOT NULL
            )
        "#,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS sync_changes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                peer_id TEXT NOT NULL,
                doc_type TEXT NOT NULL,
                change_bytes BLOB NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
