mod vec0;
pub use vec0::vec0_path;

use super::migrate;
use anyhow::Result;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::path::Path;
use std::str::FromStr;

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn new(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Get absolute path manually
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };

        // Create pool with sqlite-vec extension loaded on every connection
        let db_url = format!("sqlite://{}?mode=rwc", abs_path.display());
        // Raw release assets include the target in their filename, so do not infer the entry point.
        let connect_opts = SqliteConnectOptions::from_str(&db_url)?
            .extension_with_entrypoint(vec0_path(), "sqlite3_vec_init");
        let pool = SqlitePool::connect_with(connect_opts).await?;

        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        migrate::run(&self.pool).await
    }
}
