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
        let weights_len = weights.len();
        let biases_len = biases.len();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO model_weights (step, weights_json, biases_json, optimizer_json) VALUES (?, ?, ?, ?)"
        )
        .bind(step)
        .bind(serde_json::to_string(weights)?)
        .bind(serde_json::to_string(biases)?)
        .bind(optimizer)
        .execute(&mut *tx).await?;
        super::audit::append_tx(
            &mut tx,
            &super::audit::actor(),
            "model.save",
            &format!("step:{step}"),
            serde_json::json!({"weights": weights_len, "biases": biases_len}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn load_model(&self, step: i64) -> Result<Option<(Vec<f32>, Vec<f32>)>> {
        let row = sqlx::query("SELECT weights_json, biases_json FROM model_weights WHERE step = ? ORDER BY created_at DESC LIMIT 1")
            .bind(step)
            .fetch_optional(&self.pool).await?;

        match row {
            Some(row) => {
                let weights_json: String = row.try_get("weights_json")?;
                let biases_json: String = row.try_get("biases_json")?;
                Ok(Some((
                    serde_json::from_str(&weights_json)?,
                    serde_json::from_str(&biases_json)?,
                )))
            }
            None => Ok(None),
        }
    }

    // Full model state save/load
    pub async fn save_model_full(&self, step: i64, state_json: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO model_weights (step, weights_json, biases_json, optimizer_json) VALUES (?, ?, ?, ?)"
        )
        .bind(step)
        .bind(state_json)
        .bind(state_json)
        .bind(None::<String>)
        .execute(&mut *tx).await?;
        super::audit::append_tx(
            &mut tx,
            &super::audit::actor(),
            "model.save_full",
            &format!("step:{step}"),
            serde_json::json!({"state_bytes": state_json.len()}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn load_model_full(&self, step: i64) -> Result<Option<String>> {
        let row = sqlx::query("SELECT weights_json FROM model_weights WHERE step = ? ORDER BY created_at DESC LIMIT 1")
            .bind(step)
            .fetch_optional(&self.pool).await?;

        Ok(row.map(|r| r.get("weights_json")))
    }
}
