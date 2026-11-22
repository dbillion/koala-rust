use anyhow::Result;
use sqlx::SqlitePool;

pub(super) async fn run(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS tasks (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                body TEXT DEFAULT '',
                status TEXT DEFAULT 'not_started',
                project TEXT DEFAULT '',
                priority TEXT DEFAULT 'medium',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
