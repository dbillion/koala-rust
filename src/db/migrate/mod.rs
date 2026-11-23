use anyhow::{bail, Result};
use sqlx::SqlitePool;

mod core;
mod entities;
mod indexes;
mod meta;
mod models;
mod plugins;
mod sync;
mod tasks;
mod thoughts;
mod v1;
mod v2;
mod v3;

pub(super) const CURRENT_SCHEMA_VERSION: i64 = 3;

/// Apply numbered migrations atomically. A future schema change adds a v2
/// module and advances CURRENT_SCHEMA_VERSION; databases from newer binaries
/// are refused rather than being opened with an older schema.
pub(super) async fn run(pool: &SqlitePool) -> Result<()> {
    let current: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await?;
    if current > CURRENT_SCHEMA_VERSION {
        bail!(
            "database schema version {} is newer than this binary supports ({}); upgrade koala-rust",
            current,
            CURRENT_SCHEMA_VERSION
        );
    }
    if current == CURRENT_SCHEMA_VERSION {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    let mut version = current;
    if version < 1 {
        v1::run(&mut tx).await?;
        sqlx::query("PRAGMA user_version = 1")
            .execute(&mut *tx)
            .await?;
        version = 1;
    }
    if version < 2 {
        v2::run(&mut tx).await?;
        sqlx::query("PRAGMA user_version = 2")
            .execute(&mut *tx)
            .await?;
        version = 2;
    }
    if version < 3 {
        v3::run(&mut tx).await?;
        sqlx::query("PRAGMA user_version = 3")
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{sqlite::SqlitePoolOptions, Row};

    #[tokio::test]
    async fn numbered_migration_is_idempotent_and_refuses_newer_schema() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        run(&pool).await.unwrap();
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        let columns = sqlx::query("PRAGMA table_info(facts_tombstone)")
            .fetch_all(&pool)
            .await
            .unwrap();
        let names: Vec<String> = columns.iter().map(|row| row.get("name")).collect();
        assert!(names.contains(&"actor".to_string()));
        assert!(names.contains(&"reason".to_string()));
        let fact_columns = sqlx::query("PRAGMA table_info(facts)")
            .fetch_all(&pool)
            .await
            .unwrap();
        let fact_names: Vec<String> = fact_columns.iter().map(|row| row.get("name")).collect();
        assert!(fact_names.contains(&"expires_at".to_string()));
        run(&pool).await.unwrap();
        sqlx::query("PRAGMA user_version = 99")
            .execute(&pool)
            .await
            .unwrap();
        assert!(run(&pool)
            .await
            .unwrap_err()
            .to_string()
            .contains("newer than this binary supports"));
    }
}
