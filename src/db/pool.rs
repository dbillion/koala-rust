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
        let mut connect_opts = SqliteConnectOptions::from_str(&db_url)?;
        // SQLCipher requires the key pragma before any pragma that touches pages.
        if cfg!(feature = "sqlcipher") {
            let key = std::env::var("KOALA_DB_KEY").map_err(|_| {
                anyhow::anyhow!("KOALA_DB_KEY must be set when using the sqlcipher feature")
            })?;
            if key.is_empty() {
                anyhow::bail!("KOALA_DB_KEY must not be empty");
            }
            connect_opts = connect_opts.pragma("key", format!("'{}'", key.replace('\'', "''")));
        }
        connect_opts = connect_opts
            .pragma("secure_delete", "ON")
            .extension_with_entrypoint(vec0_path(), "sqlite3_vec_init");
        let pool = SqlitePool::connect_with(connect_opts).await?;

        let db = Self { pool };
        let has_existing_data = abs_path.exists()
            && std::fs::metadata(&abs_path)
                .map(|meta| meta.len() > 0)
                .unwrap_or(false);
        let schema_version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&db.pool)
            .await?;
        if schema_version > migrate::CURRENT_SCHEMA_VERSION {
            anyhow::bail!(
                "database schema version {} is newer than this binary supports ({})",
                schema_version,
                migrate::CURRENT_SCHEMA_VERSION
            );
        }
        if has_existing_data && schema_version < migrate::CURRENT_SCHEMA_VERSION {
            use std::time::{SystemTime, UNIX_EPOCH};
            let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
            let backup = abs_path.with_file_name(format!(
                "{}.pre-migration-v{}-{}.bak",
                abs_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("koala.db"),
                migrate::CURRENT_SCHEMA_VERSION,
                stamp
            ));
            db.backup_to(&backup).await?;
            tracing::info!(
                "Created pre-migration database backup at {}",
                backup.display()
            );
        }
        db.migrate().await?;
        db.configure_incremental_vacuum().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        migrate::run(&self.pool).await
    }

    /// Convert legacy databases once to incremental auto-vacuum. SQLite only
    /// applies the setting to an existing schema after VACUUM rebuilds it.
    async fn configure_incremental_vacuum(&self) -> Result<()> {
        let mode: i64 = sqlx::query_scalar("PRAGMA auto_vacuum")
            .fetch_one(&self.pool)
            .await?;
        if mode == 0 {
            let mut conn = self.pool.acquire().await?;
            sqlx::query("PRAGMA auto_vacuum=INCREMENTAL")
                .execute(&mut *conn)
                .await?;
            sqlx::query("VACUUM").execute(&mut *conn).await?;
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "sqlcipher"))]
mod sqlcipher_tests {
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
}
