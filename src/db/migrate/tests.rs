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
