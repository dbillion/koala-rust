use super::Database;

#[tokio::test]
async fn encrypted_database_requires_the_correct_key_and_hides_plaintext() {
    let root = std::env::temp_dir().join(format!("koala-sqlcipher-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("encrypted.db");
    let previous_key = std::env::var_os("KOALA_DB_KEY");
    std::env::set_var("KOALA_DB_KEY", "test-key");
    let db = Database::new(&path).await.unwrap();
    db.write_fact("secret", "sqlcipher-plaintext-marker")
        .await
        .unwrap();
    db.pool.close().await;

    let bytes = std::fs::read(&path).unwrap();
    assert!(!bytes
        .windows(b"sqlcipher-plaintext-marker".len())
        .any(|window| window == b"sqlcipher-plaintext-marker"));

    std::env::set_var("KOALA_DB_KEY", "wrong-key");
    assert!(Database::new(&path).await.is_err());

    std::env::set_var("KOALA_DB_KEY", "test-key");
    let reopened = Database::new(&path).await.unwrap();
    let (_, value) = reopened.read_fact("secret").await.unwrap().unwrap();
    assert_eq!(value, "sqlcipher-plaintext-marker");
    reopened.pool.close().await;

    match previous_key {
        Some(value) => std::env::set_var("KOALA_DB_KEY", value),
        None => std::env::remove_var("KOALA_DB_KEY"),
    }
    let _ = std::fs::remove_dir_all(root);
}
