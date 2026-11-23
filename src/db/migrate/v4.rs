use anyhow::Result;
use sqlx::{Sqlite, Transaction};

pub(super) async fn run(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
    sqlx::query("CREATE TABLE IF NOT EXISTS audit_log (id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp TEXT NOT NULL, actor TEXT NOT NULL, action TEXT NOT NULL, subject TEXT NOT NULL, details_json TEXT NOT NULL, previous_hash TEXT NOT NULL, entry_hash TEXT NOT NULL UNIQUE)")
        .execute(&mut **tx).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS audit_head (singleton INTEGER PRIMARY KEY CHECK(singleton = 1), head_hash TEXT NOT NULL, key_id TEXT, public_key BLOB, signature BLOB, updated_at TEXT NOT NULL)")
        .execute(&mut **tx).await?;
    sqlx::query("INSERT OR IGNORE INTO audit_head (singleton, head_hash, updated_at) VALUES (1, ?, CURRENT_TIMESTAMP)")
        .bind("0000000000000000000000000000000000000000000000000000000000000000")
        .execute(&mut **tx).await?;
    sqlx::query("CREATE TRIGGER IF NOT EXISTS audit_log_no_update BEFORE UPDATE ON audit_log BEGIN SELECT RAISE(ABORT, 'audit log is append-only'); END")
        .execute(&mut **tx).await?;
    sqlx::query("CREATE TRIGGER IF NOT EXISTS audit_log_no_delete BEFORE DELETE ON audit_log BEGIN SELECT RAISE(ABORT, 'audit log is append-only'); END")
        .execute(&mut **tx).await?;
    Ok(())
}
