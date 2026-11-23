use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::test]
async fn encrypted_database_requires_the_correct_key_and_hides_plaintext() {
    let root = std::env::temp_dir().join(format!("koala-sqlcipher-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("encrypted.db");
    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .pragma("key", "'test-key'");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE secrets (value TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO secrets VALUES ('sqlcipher-plaintext-marker')")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let bytes = std::fs::read(&path).unwrap();
    assert!(!bytes
        .windows(b"sqlcipher-plaintext-marker".len())
        .any(|window| window == b"sqlcipher-plaintext-marker"));

    let wrong = SqliteConnectOptions::new()
        .filename(&path)
        .read_only(true)
        .pragma("key", "'wrong-key'");
    let wrong_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(wrong)
        .await
        .unwrap();
    assert!(sqlx::query("SELECT value FROM secrets")
        .fetch_one(&wrong_pool)
        .await
        .is_err());
    wrong_pool.close().await;

    let correct = SqliteConnectOptions::new()
        .filename(&path)
        .read_only(true)
        .pragma("key", "'test-key'");
    let correct_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(correct)
        .await
        .unwrap();
    let value: String = sqlx::query_scalar("SELECT value FROM secrets")
        .fetch_one(&correct_pool)
        .await
        .unwrap();
    assert_eq!(value, "sqlcipher-plaintext-marker");
    correct_pool.close().await;
    let _ = std::fs::remove_dir_all(root);
}
