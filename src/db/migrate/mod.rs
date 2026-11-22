use anyhow::Result;
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

pub(super) async fn run(pool: &SqlitePool) -> Result<()> {
    core::run(pool).await?;
    entities::run(pool).await?;
    tasks::run(pool).await?;
    models::run(pool).await?;
    thoughts::run(pool).await?;
    sync::run(pool).await?;
    meta::run(pool).await?;
    plugins::run(pool).await?;
    indexes::run(pool).await?;
    Ok(())
}
