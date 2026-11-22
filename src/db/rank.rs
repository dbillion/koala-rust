use super::{Database, RankExplanation};
use anyhow::Result;
use sqlx::Row;

impl Database {
    pub async fn explain_rank(&self, name: &str) -> Result<Option<RankExplanation>> {
        let row = sqlx::query("SELECT name, signal_type, value FROM rank_signals WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| RankExplanation {
            name: r.get("name"),
            signal: r.get("signal_type"),
            score: r.get("value"),
        }))
    }
}
