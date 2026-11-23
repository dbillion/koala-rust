mod init;
mod vec0;
pub use vec0::vec0_path;

#[cfg(all(test, feature = "sqlcipher"))]
mod sqlcipher_tests;

use sqlx::SqlitePool;

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}
