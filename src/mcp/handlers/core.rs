use super::super::KoalaServer;
use anyhow::Result;
use serde_json::Value;

impl KoalaServer {
    pub(in crate::mcp) async fn handle_health(&self, _args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let health = brain.db.health().await?;
        Ok(serde_json::to_value(health)?)
    }
}
