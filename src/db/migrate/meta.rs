use anyhow::Result;
use sqlx::SqlitePool;

pub(super) async fn run(pool: &SqlitePool) -> Result<()> {
    // Small key/value store: embedding model identity and the reindex-pending marker.
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS brain_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )
        "#,
    )
    .execute(pool)
    .await?;

    // v0.4: Embedding metadata.
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS embedding_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_type TEXT NOT NULL, -- 'fact' | 'entity'
                source_id INTEGER NOT NULL,
                model_name TEXT NOT NULL,
                dimension INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
