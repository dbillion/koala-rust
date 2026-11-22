use super::super::{missing_arg, KoalaServer};
use anyhow::Result;
use serde_json::Value;

impl KoalaServer {
    pub(in crate::mcp) async fn handle_get_user_context(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let ctx = brain.get_user_context().await?;
        Ok(serde_json::to_value(ctx)?)
    }

    pub(in crate::mcp) async fn handle_get_project_context(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let project = args["project"]
            .as_str()
            .ok_or_else(|| missing_arg("project"))?;
        let ctx = brain.get_project_context(project).await?;
        Ok(serde_json::to_value(ctx)?)
    }

    pub(in crate::mcp) async fn handle_bootstrap_context(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let project = args["project"]
            .as_str()
            .ok_or_else(|| missing_arg("project"))?;
        let ctx = brain.bootstrap_context(project).await?;
        Ok(serde_json::to_value(ctx)?)
    }

    pub(in crate::mcp) async fn handle_record_decision(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let project = args["project"]
            .as_str()
            .ok_or_else(|| missing_arg("project"))?;
        let decision = args["decision"]
            .as_str()
            .ok_or_else(|| missing_arg("decision"))?;
        let rationale = args["rationale"].as_str();
        let res = brain.record_decision(project, decision, rationale).await?;
        Ok(res)
    }

    pub(in crate::mcp) async fn handle_update_project_status(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let project = args["project"]
            .as_str()
            .ok_or_else(|| missing_arg("project"))?;
        let status = args["status"]
            .as_str()
            .ok_or_else(|| missing_arg("status"))?;
        let blockers: Vec<String> = args["blockers"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let next_step = args["next_step"].as_str();
        let res = brain
            .update_project_status(project, status, &blockers, next_step)
            .await?;
        Ok(res)
    }

    pub(in crate::mcp) async fn handle_create_handoff(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let project = args["project"]
            .as_str()
            .ok_or_else(|| missing_arg("project"))?;
        let summary = args["summary"]
            .as_str()
            .ok_or_else(|| missing_arg("summary"))?;
        let next_step = args["next_step"]
            .as_str()
            .ok_or_else(|| missing_arg("next_step"))?;
        let blockers: Vec<String> = args["blockers"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let cp = brain
            .create_handoff(project, summary, next_step, &blockers)
            .await?;
        Ok(serde_json::to_value(cp)?)
    }

    pub(in crate::mcp) async fn handle_search_memory(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let query = args["query"].as_str().ok_or_else(|| missing_arg("query"))?;
        let project = args["project"].as_str();
        let limit = args["limit"].as_u64().map(|v| v as usize);
        let results = brain.search_memory(query, project, limit).await?;
        Ok(serde_json::to_value(results)?)
    }
}
