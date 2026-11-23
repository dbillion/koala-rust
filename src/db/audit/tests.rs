use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn audit_chain_verifies_and_detects_tampering() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    crate::db::migrate::run(&pool).await.unwrap();
    let db = crate::db::Database { pool: pool.clone() };
    db.write_fact("audit:test", "secret fact").await.unwrap();
    let verified = db.audit_verify().await.unwrap();
    assert_eq!(verified.entries, 2);
    assert!(!verified.signed);
    assert!(
        sqlx::query("UPDATE audit_log SET action='tampered' WHERE id=1")
            .execute(&pool)
            .await
            .is_err()
    );
    sqlx::query("DROP TRIGGER audit_log_no_update")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE audit_log SET action='tampered' WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    let error = db.audit_verify().await.unwrap_err().to_string();
    assert!(
        error.contains("row 1"),
        "unexpected audit verification error: {error}"
    );
}

#[tokio::test]
async fn audit_records_cover_database_mutation_families() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    crate::db::migrate::run(&pool).await.unwrap();
    let db = crate::db::Database { pool: pool.clone() };

    db.meta_set("audit-test", "metadata").await.unwrap();
    db.save_model(1, &[0.1, 0.2], &[0.3], None).await.unwrap();
    db.save_model_full(2, r#"{"weights":[1]}"#).await.unwrap();
    db.feedback("audit-test", "positive", "recall")
        .await
        .unwrap();
    db.add_thought(3, "private thought").await.unwrap();
    assert_eq!(db.clear_thoughts().await.unwrap(), 1);
    db.add_conclusion("private conclusion").await.unwrap();

    let records = db.audit_export().await.unwrap();
    let actions: Vec<&str> = records
        .iter()
        .map(|record| record.action.as_str())
        .collect();
    for expected in [
        "schema.migrate",
        "meta.set",
        "model.save",
        "model.save_full",
        "feedback.record",
        "thought.create",
        "thought.clear",
        "conclusion.create",
    ] {
        assert!(
            actions.contains(&expected),
            "missing audit action: {expected}"
        );
    }
    assert_eq!(db.audit_verify().await.unwrap().entries, actions.len());
    let serialized = serde_json::to_string(&records).unwrap();
    assert!(!serialized.contains("private thought"));
    assert!(!serialized.contains("private conclusion"));
}
