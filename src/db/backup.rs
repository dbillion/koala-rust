use super::Database;
use anyhow::{bail, Result};
use std::path::Path;

impl Database {
    /// Create a consistent SQLite snapshot using the online VACUUM INTO API.
    /// The destination must not already exist.
    pub async fn backup_to(&self, path: &Path) -> Result<()> {
        if path.exists() {
            bail!("backup destination already exists: {}", path.display());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // SQLite's VACUUM INTO grammar does not bind the filename as a normal
        // query parameter. Quote it as a SQL string literal, escaping apostrophes.
        let filename = path.to_string_lossy().replace(char::from(39), "''");
        let statement = format!("VACUUM INTO '{}';", filename);
        sqlx::raw_sql(&statement).execute(&self.pool).await?;
        if !path.is_file() {
            bail!(
                "SQLite reported a successful backup but did not create {}",
                path.display()
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    #[tokio::test]
    async fn backup_to_creates_consistent_snapshot_and_refuses_overwrite() {
        let root = std::env::temp_dir().join(format!("koala-backup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.db");
        let source_options = SqliteConnectOptions::new()
            .filename(&source_path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(source_options)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE facts (name TEXT, text TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO facts VALUES ('kept','backup data')")
            .execute(&pool)
            .await
            .unwrap();
        let db = Database { pool };
        let path = root.join("backup.db");
        db.backup_to(&path).await.unwrap();
        assert!(db.backup_to(&path).await.is_err());

        let options = SqliteConnectOptions::new().filename(&path).read_only(true);
        let backup = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();
        let text: String = sqlx::query_scalar("SELECT text FROM facts WHERE name='kept'")
            .fetch_one(&backup)
            .await
            .unwrap();
        assert_eq!(text, "backup data");
        backup.close().await;
        let _ = std::fs::remove_dir_all(root);
    }
}
