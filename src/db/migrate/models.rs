use anyhow::Result;
use sqlx::{Sqlite, Transaction};

pub(super) async fn run(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
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
    .execute(&mut **tx)
    .await?;
    Ok(())
}
