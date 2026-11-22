use super::Database;
use anyhow::Result;
use sqlx::Row;

impl Database {
    // ===== MODEL PERSISTENCE =====
    pub async fn save_model(
        &self,
        step: i64,
        weights: &[f32],
        biases: &[f32],
        optimizer: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO model_weights (step, weights_json, biases_json, optimizer_json) VALUES (?, ?, ?, ?)"
        )
        .bind(step)
        .bind(serde_json::to_string(weights)?)
        .bind(serde_json::to_string(biases)?)
        .bind(optimizer)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn load_model(&self, step: i64) -> Result<Option<(Vec<f32>, Vec<f32>)>> {
        let row = sqlx::query("SELECT weights_json, biases_json FROM model_weights WHERE step = ? ORDER BY created_at DESC LIMIT 1")
            .bind(step)
            .fetch_optional(&self.pool).await?;

        Ok(row.map(|r| {
            (
                serde_json::from_str(r.get("weights_json")).unwrap(),
                serde_json::from_str(r.get("biases_json")).unwrap(),
            )
        }))
    }

    // Full model state save/load
    pub async fn save_model_full(&self, step: i64, state_json: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO model_weights (step, weights_json, biases_json, optimizer_json) VALUES (?, ?, ?, ?)"
        )
        .bind(step)
        .bind(state_json)
        .bind(state_json)
        .bind(None::<String>)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn load_model_full(&self, step: i64) -> Result<Option<String>> {
        let row = sqlx::query("SELECT weights_json FROM model_weights WHERE step = ? ORDER BY created_at DESC LIMIT 1")
            .bind(step)
            .fetch_optional(&self.pool).await?;

        Ok(row.map(|r| r.get("weights_json")))
    }
}
