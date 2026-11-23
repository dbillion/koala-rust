use anyhow::Result;
use sqlx::{Sqlite, Transaction};

pub(super) async fn run(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS thoughts (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                step INTEGER NOT NULL,
                thought TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS conclusions (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                summary TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS rank_signals (
                name TEXT PRIMARY KEY,
                signal_type TEXT NOT NULL,
                value REAL NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
