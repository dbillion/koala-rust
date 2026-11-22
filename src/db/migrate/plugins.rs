use anyhow::Result;
use sqlx::SqlitePool;

// v0.5: Plugin metadata.
pub(super) async fn run(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS plugin_registry (
                name TEXT PRIMARY KEY,
                version TEXT NOT NULL,
                manifest_json TEXT NOT NULL,
                plugin_type TEXT NOT NULL, -- 'wasm' | 'native'
                path TEXT NOT NULL,
                enabled BOOLEAN DEFAULT 1,
                loaded_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
