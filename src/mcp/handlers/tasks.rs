use super::super::{missing_arg, KoalaServer};
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_create_task(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let title = args["title"].as_str().ok_or_else(|| missing_arg("title"))?;
        let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
        let status = args
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("not_started");
        let project = args.get("project").and_then(|v| v.as_str()).unwrap_or("");
        let priority = args
            .get("priority")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");
        let rowid = brain
            .db
            .create_task(title, body, status, project, priority)
            .await?;
        Ok(json!({"id": rowid, "title": title, "status": status}))
    }

    pub(in crate::mcp) async fn handle_query_tasks(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let status = args.get("status").and_then(|v| v.as_str());
        let project = args.get("project").and_then(|v| v.as_str());
        let search = args.get("search").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);
        let tasks = brain.db.query_tasks(status, project, search, limit).await?;
        Ok(json!({"tasks": tasks}))
    }

    pub(in crate::mcp) async fn handle_update_task(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let id = args["id"].as_i64().ok_or_else(|| missing_arg("id"))?;
        let title = args.get("title").and_then(|v| v.as_str());
        let body = args.get("body").and_then(|v| v.as_str());
        let status = args.get("status").and_then(|v| v.as_str());
        let project = args.get("project").and_then(|v| v.as_str());
        let priority = args.get("priority").and_then(|v| v.as_str());
        let updated = brain
            .db
            .update_task(id, title, body, status, project, priority)
            .await?;
        Ok(json!({"updated": updated}))
    }
}
