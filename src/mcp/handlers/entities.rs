use super::super::{missing_arg, KoalaServer};
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_create_entities(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        let type_ = args.get("type").and_then(|v| v.as_str());
        let project = args.get("project").and_then(|v| v.as_str());
        let observations = args
            .get("observations")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();
        let rowid = brain
            .db
            .create_entity(name, type_, project, &observations)
            .await?;
        Ok(json!({"rowid": rowid, "name": name, "created": true}))
    }

    pub(in crate::mcp) async fn handle_add_observations(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        let observations = args["observations"]
            .as_array()
            .ok_or_else(|| missing_arg("observations"))?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<String>>();
        let added = brain.db.add_observations(name, &observations).await?;
        Ok(json!({"rowid": added, "name": name, "added": added}))
    }

    pub(in crate::mcp) async fn handle_open_nodes(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let names = args["names"]
            .as_array()
            .ok_or_else(|| missing_arg("names"))?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<String>>();
        let nodes = brain.db.open_nodes(&names).await?;
        Ok(json!({"nodes": nodes}))
    }

    pub(in crate::mcp) async fn handle_search_nodes(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let query = args["query"].as_str().ok_or_else(|| missing_arg("query"))?;
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);
        let results = brain.db.search_nodes(query, limit).await?;
        Ok(json!({"results": results}))
    }

    pub(in crate::mcp) async fn handle_create_relations(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let from = args["from"].as_str().ok_or_else(|| missing_arg("from"))?;
        let to = args["to"].as_str().ok_or_else(|| missing_arg("to"))?;
        let rel_type = args["type"].as_str().ok_or_else(|| missing_arg("type"))?;
        let rowid = brain.db.create_relation(from, to, rel_type).await?;
        Ok(json!({"rowid": rowid, "from": from, "to": to, "type": rel_type}))
    }
}
