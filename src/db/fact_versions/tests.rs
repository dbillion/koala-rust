use super::*;
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn point_in_time_reads_select_latest_version_before_timestamp() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("CREATE TABLE facts (rowid INTEGER PRIMARY KEY, name TEXT, text TEXT, created_at DATETIME, is_current BOOLEAN, expires_at DATETIME)")
            .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO facts (name,text,created_at,is_current) VALUES ('profile','old','2025-01-01 00:00:00',0),('profile','new','2025-02-01 00:00:00',1),('other','later','2025-03-01 00:00:00',1)")
            .execute(&pool).await.unwrap();
    let db = Database { pool: pool.clone() };

    let old = db
        .read_fact_as_of("profile", "2025-01-15T00:00:00Z")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(old.1, "old");
    let facts = db.facts_as_of("2025-01-15 00:00:00").await.unwrap();
    assert_eq!(
        facts,
        vec![(old.0, "profile".to_string(), "old".to_string())]
    );
    assert!(db.read_fact_as_of("profile", "not-a-date").await.is_err());
}
