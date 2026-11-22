use super::super::{missing_arg, KoalaServer};
use crate::sync;
use anyhow::Result;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_sync_write_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        let text = args["text"].as_str().ok_or_else(|| missing_arg("text"))?;
        let id = brain.sync_write_fact(name, text).await?;
        Ok(json!({"ok": true, "id": id}))
    }

    pub(in crate::mcp) async fn handle_sync_read_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        let fact = brain.sync_read_fact(name).await?;
        Ok(fact
            .map(|(id, text)| json!({"id": id, "text": text}))
            .unwrap_or_else(|| json!({"error": "not found"})))
    }

    pub(in crate::mcp) async fn handle_sync_list_facts(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let facts = brain.sync_list_facts().await?;
        Ok(json!(facts))
    }

    pub(in crate::mcp) async fn handle_sync_create_entity(&self, args: Value) -> Result<Value> {
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
        let id = brain
            .sync_create_entity(name, type_, project, &observations)
            .await?;
        Ok(json!({"ok": true, "id": id}))
    }

    pub(in crate::mcp) async fn handle_sync_get_changes(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let doc_type_str = args["doc_type"]
            .as_str()
            .ok_or_else(|| missing_arg("doc_type"))?;
        let doc_type = match doc_type_str {
            "facts" => sync::SyncDocType::Facts,
            "entities" => sync::SyncDocType::Entities,
            "relations" => sync::SyncDocType::Relations,
            "tasks" => sync::SyncDocType::Tasks,
            "thoughts" => sync::SyncDocType::Thoughts,
            "model" => sync::SyncDocType::Model,
            _ => return Err(anyhow::anyhow!("Invalid doc_type: {}", doc_type_str)),
        };
        let changes = brain.sync_get_changes(doc_type).await?;
        let changes_b64: Vec<String> = changes.iter().map(|c| STANDARD.encode(c)).collect();
        Ok(json!({"changes": changes_b64}))
    }

    pub(in crate::mcp) async fn handle_sync_merge_changes(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let doc_type_str = args["doc_type"]
            .as_str()
            .ok_or_else(|| missing_arg("doc_type"))?;
        let doc_type = match doc_type_str {
            "facts" => sync::SyncDocType::Facts,
            "entities" => sync::SyncDocType::Entities,
            "relations" => sync::SyncDocType::Relations,
            "tasks" => sync::SyncDocType::Tasks,
            "thoughts" => sync::SyncDocType::Thoughts,
            "model" => sync::SyncDocType::Model,
            _ => return Err(anyhow::anyhow!("Invalid doc_type: {}", doc_type_str)),
        };
        let changes_b64: Vec<String> = args["changes"]
            .as_array()
            .ok_or_else(|| missing_arg("changes"))?
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        let changes: Vec<Vec<u8>> = changes_b64
            .iter()
            .filter_map(|s| STANDARD.decode(s).ok())
            .collect();
        brain.sync_merge_changes(doc_type, &changes).await?;
        Ok(json!({"ok": true}))
    }
}
