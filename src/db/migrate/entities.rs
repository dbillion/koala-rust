use anyhow::Result;
use sqlx::{Sqlite, Transaction};

pub(super) async fn run(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS entities (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                type TEXT,
                project TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS observations (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                entity_name TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (entity_name) REFERENCES entities(name) ON DELETE CASCADE
            )
        "#,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS relations (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                from_entity TEXT NOT NULL,
                to_entity TEXT NOT NULL,
                rel_type TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (from_entity) REFERENCES entities(name) ON DELETE CASCADE,
                FOREIGN KEY (to_entity) REFERENCES entities(name) ON DELETE CASCADE
            )
        "#,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
