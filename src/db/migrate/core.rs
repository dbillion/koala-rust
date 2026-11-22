use anyhow::Result;
use sqlx::SqlitePool;

pub(super) async fn run(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS facts (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                text TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_current BOOLEAN DEFAULT 1
            )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS facts_tombstone (
                name TEXT PRIMARY KEY,
                deleted_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
