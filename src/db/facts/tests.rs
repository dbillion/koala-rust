use super::*;
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn delete_fact_purges_all_versions_and_keeps_text_free_tombstone() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("CREATE TABLE facts (rowid INTEGER PRIMARY KEY, name TEXT, text TEXT, is_current BOOLEAN, expires_at DATETIME)")
            .execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE facts_tombstone (name TEXT PRIMARY KEY, deleted_at DATETIME DEFAULT CURRENT_TIMESTAMP, actor TEXT NOT NULL DEFAULT 'local', reason TEXT NOT NULL DEFAULT 'user_requested')")
            .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO facts (name, text, is_current) VALUES ('private', 'old secret', 0), ('private', 'new secret', 1)")
            .execute(&pool).await.unwrap();
    let db = Database { pool: pool.clone() };

    assert!(db.delete_fact("private").await.unwrap());
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM facts WHERE name='private'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let tombstone: String =
        sqlx::query_scalar("SELECT name FROM facts_tombstone WHERE name='private'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
    assert_eq!(tombstone, "private");
    let audit: (String, String) =
        sqlx::query_as("SELECT actor, reason FROM facts_tombstone WHERE name='private'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(audit, ("local".to_string(), "user_requested".to_string()));
    assert!(!db.delete_fact("private").await.unwrap());
}

#[cfg(test)]
mod ttl_tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn expired_current_fact_purges_history_and_records_tombstone() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE facts (rowid INTEGER PRIMARY KEY, name TEXT, text TEXT, is_current BOOLEAN, expires_at DATETIME)").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE facts_tombstone (name TEXT PRIMARY KEY, deleted_at DATETIME DEFAULT CURRENT_TIMESTAMP, actor TEXT NOT NULL DEFAULT 'local', reason TEXT NOT NULL DEFAULT 'user_requested')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO facts (name,text,is_current,expires_at) VALUES ('session','old',0,datetime('now','-2 days')),('session','new',1,datetime('now','-1 second'))").execute(&pool).await.unwrap();
        let db = Database { pool: pool.clone() };
        let rowids = db.purge_expired_facts().await.unwrap();
        assert_eq!(rowids.len(), 2);
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM facts WHERE name='session'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let reason: String =
            sqlx::query_scalar("SELECT reason FROM facts_tombstone WHERE name='session'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(remaining, 0);
        assert_eq!(reason, "ttl_expired");
    }
}
