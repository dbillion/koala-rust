use super::super::{missing_arg, KoalaServer};
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_model_save(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let model = brain.model.lock().await;
        let model_ref = model
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no model loaded"))?;
        let step = args["step"].as_i64().ok_or_else(|| missing_arg("step"))?;
        let state = model_ref.to_state();
        let state_json = serde_json::to_string(&state)?;
        brain.db.save_model_full(step, &state_json).await?;
        Ok(json!({"ok": true, "step": step}))
    }

    pub(in crate::mcp) async fn handle_model_load(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let step = args["step"].as_i64().ok_or_else(|| missing_arg("step"))?;
        let state_json: Option<String> = brain.db.load_model_full(step).await?;
        if let Some(json_str) = state_json {
            let state: crate::model::ModelState = serde_json::from_str(&json_str)?;
            Ok(json!({
                "ok": true,
                "step": step,
                "state": state,
                "note": "Model state retrieved. Use model_init with same hidden/activation to re-initialize."
            }))
        } else {
            Ok(json!({"ok": false, "error": "no model found at step"}))
        }
    }
}
