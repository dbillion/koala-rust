use super::super::KoalaServer;
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_model_status(&self, _args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let model = brain.model.lock().await;
        let status = if let Some(model_ref) = model.as_ref() {
            serde_json::to_value(model_ref.status())?
        } else {
            json!({
                "initialized": false,
                "hidden": 0,
                "activation": "none",
                "step": 0,
                "last_loss": null,
                "allocated": 0,
                "committed": 0,
                "backend": "pure-rust",
                "gpu": "unavailable",
                "gpu_reason": "no CUDA; CPU reference is the supported backend",
                "rank": {
                    "total_entities": 0,
                    "total_relations": 0,
                    "total_observations": 0,
                    "total_facts": 0,
                    "total_tasks": 0
                }
            })
        };
        Ok(status)
    }
}
