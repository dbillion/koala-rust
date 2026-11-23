use anyhow::{bail, Result};
use sqlx::SqlitePool;

mod core;
mod entities;
mod indexes;
mod meta;
mod models;
mod plugins;
mod sync;
mod tasks;
mod thoughts;
mod v1;
mod v2;
mod v3;
mod v4;

pub(super) const CURRENT_SCHEMA_VERSION: i64 = 4;

/// Apply numbered migrations atomically. A future schema change adds a v2
/// module and advances CURRENT_SCHEMA_VERSION; databases from newer binaries
/// are refused rather than being opened with an older schema.
pub(super) async fn run(pool: &SqlitePool) -> Result<()> {
    let current: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await?;
    if current > CURRENT_SCHEMA_VERSION {
        bail!(
            "database schema version {} is newer than this binary supports ({}); upgrade koala-rust",
            current,
            CURRENT_SCHEMA_VERSION
        );
    }
    if current == CURRENT_SCHEMA_VERSION {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    let mut version = current;
    if version < 1 {
        v1::run(&mut tx).await?;
        sqlx::query("PRAGMA user_version = 1")
            .execute(&mut *tx)
            .await?;
        version = 1;
    }
    if version < 2 {
        v2::run(&mut tx).await?;
        sqlx::query("PRAGMA user_version = 2")
            .execute(&mut *tx)
            .await?;
        version = 2;
    }
    if version < 3 {
        v3::run(&mut tx).await?;
        sqlx::query("PRAGMA user_version = 3")
            .execute(&mut *tx)
            .await?;
        version = 3;
    }
    if version < 4 {
        v4::run(&mut tx).await?;
        sqlx::query("PRAGMA user_version = 4")
            .execute(&mut *tx)
            .await?;
        super::audit::append_tx(
            &mut tx,
            "system",
            "schema.migrate",
            "database",
            serde_json::json!({"from": current, "to": 4}),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests;
