use super::super::{missing_arg, KoalaServer};
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_write_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        let text = args["text"].as_str().ok_or_else(|| missing_arg("text"))?;
        let rowid = brain.db.write_fact(name, text).await?;
        Ok(json!({"rowid": rowid, "name": name}))
    }

    pub(in crate::mcp) async fn handle_read_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        let fact = brain.db.read_fact(name).await?;
        Ok(fact
            .map(|(rowid, text)| json!({"rowid": rowid, "name": name, "body": text}))
            .unwrap_or_else(|| json!({"error": "not found"})))
    }

    pub(in crate::mcp) async fn handle_list_facts(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let facts = brain.db.list_facts().await?;
        Ok(json!(facts))
    }

    pub(in crate::mcp) async fn handle_read_fact_versions(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);
        let versions = brain.db.read_fact_versions(name, limit).await?;
        Ok(json!(versions))
    }

    pub(in crate::mcp) async fn handle_delete_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        let deleted = brain.delete_fact_and_vector(name).await?;
        Ok(json!({"deleted": deleted}))
    }

    pub(in crate::mcp) async fn handle_delete_fact_prefix(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let prefix = args["prefix"]
            .as_str()
            .ok_or_else(|| missing_arg("prefix"))?;
        let count = brain.delete_fact_prefix_and_vectors(prefix).await?;
        Ok(json!({"deleted": count}))
    }
}
