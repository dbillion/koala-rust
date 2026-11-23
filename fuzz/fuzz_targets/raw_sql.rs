#![no_main]
use koala_rust::db::Database;
use libfuzzer_sys::fuzz_target;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fuzz_target!(|data: &[u8]| {
    if data.is_empty() || data.len() > 4096 {
        return;
    }
    let Ok(query) = std::str::from_utf8(data) else {
        return;
    };
    let first = query.trim_start().to_ascii_lowercase();
    if first.starts_with("attach")
        || first.starts_with("detach")
        || first.starts_with("vacuum")
        || first.starts_with("pragma")
    {
        return;
    }

    let runtime = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("fuzz runtime")
    });
    runtime.block_on(async {
        let Ok(pool) = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
        else {
            return;
        };
        let _ = sqlx::query("CREATE TABLE fuzz_data (id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&pool)
            .await;
        let db = Database { pool };
        let _ = db.raw_sql(query).await;
    });
});
