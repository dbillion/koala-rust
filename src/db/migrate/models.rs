use anyhow::Result;
use sqlx::SqlitePool;

pub(super) async fn run(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS model_weights (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                step INTEGER NOT NULL,
                weights_json TEXT NOT NULL,
                biases_json TEXT NOT NULL,
                optimizer_json TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
