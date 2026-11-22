use super::migrate;
use anyhow::Result;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Path of the sqlite-vec extension. Override with env KOALA_VEC0_PATH.
/// On non-Linux platforms the extension is loaded from the directory of the
/// running binary so the release ships `vec0.dll` / `vec0.dylib` next to it;
/// Linux keeps the traditional system path.
pub fn vec0_path() -> String {
    std::env::var("KOALA_VEC0_PATH")
        .ok()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| {
            if cfg!(target_os = "windows") || cfg!(target_os = "macos") {
                let exe = std::env::current_exe()
                    .unwrap_or_else(|_| PathBuf::from("/usr/local/lib/vec0.so"));
                let dir = exe.parent().unwrap_or_else(|| Path::new("/usr/local/lib"));
                let ext = if cfg!(target_os = "windows") {
                    "vec0.dll"
                } else {
                    "vec0.dylib"
                };
                dir.join(ext).to_string_lossy().to_string()
            } else {
                PathBuf::from("/usr/local/lib/vec0.so")
                    .to_string_lossy()
                    .to_string()
            }
        })
}

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
        let connect_opts = SqliteConnectOptions::from_str(&db_url)?.extension(vec0_path());
        let pool = SqlitePool::connect_with(connect_opts).await?;

        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        migrate::run(&self.pool).await
    }
}
