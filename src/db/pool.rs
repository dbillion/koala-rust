use super::migrate;
use anyhow::Result;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Path of the sqlite-vec extension. Override with env KOALA_VEC0_PATH.
/// Release bundles place the matching `vec0.*` library beside the executable.
/// Linux retains the system path as a fallback for existing installations.
fn adjacent_vec0_path(executable: &Path, filename: &str) -> Option<PathBuf> {
    let candidate = executable.parent()?.join(filename);
    candidate.is_file().then_some(candidate)
}

pub fn vec0_path() -> String {
    if let Some(path) = std::env::var("KOALA_VEC0_PATH")
        .ok()
        .filter(|path| !path.trim().is_empty())
    {
        return path;
    }

    let filename = if cfg!(target_os = "windows") {
        "vec0.dll"
    } else if cfg!(target_os = "macos") {
        "vec0.dylib"
    } else {
        "vec0.so"
    };
    let executable = std::env::current_exe().ok();
    let bundled = executable
        .as_deref()
        .and_then(|executable| adjacent_vec0_path(executable, filename));
    if let Some(path) = bundled {
        return path.to_string_lossy().to_string();
    }

    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/usr/local/lib/vec0.so")
            .to_string_lossy()
            .to_string()
    }

    #[cfg(not(target_os = "linux"))]
    {
        executable
            .as_deref()
            .and_then(|executable| executable.parent().map(|dir| dir.join(filename)))
            .unwrap_or_else(|| PathBuf::from(filename))
            .to_string_lossy()
            .to_string()
    }
}

#[cfg(test)]
mod vec0_path_tests {
    use super::adjacent_vec0_path;
    use std::fs;

    #[test]
    fn finds_bundled_extension_next_to_executable() {
        let directory =
            std::env::temp_dir().join(format!("koala-vec0-path-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let executable = directory.join("koala-rust");
        let extension = directory.join("vec0.so");
        fs::write(&extension, b"test extension").unwrap();

        assert_eq!(
            adjacent_vec0_path(&executable, "vec0.so"),
            Some(extension.clone())
        );

        fs::remove_file(extension).unwrap();
        fs::remove_dir(directory).unwrap();
    }
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
