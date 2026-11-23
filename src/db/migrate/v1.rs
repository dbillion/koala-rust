use anyhow::Result;
use sqlx::{Sqlite, Transaction};

/// Initial schema migration. Legacy databases have user_version=0; all DDL is
/// idempotent so their existing data and objects are preserved.
pub(super) async fn run(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
    super::core::run(tx).await?;
    super::entities::run(tx).await?;
    super::tasks::run(tx).await?;
    super::models::run(tx).await?;
    super::thoughts::run(tx).await?;
    super::sync::run(tx).await?;
    super::meta::run(tx).await?;
    super::plugins::run(tx).await?;
    super::indexes::run(tx).await?;
    Ok(())
}
