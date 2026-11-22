use super::super::{missing_arg, KoalaServer};
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_feedback(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        let signal = args["signal"]
            .as_str()
            .ok_or_else(|| missing_arg("signal"))?;
        let kind = args["kind"].as_str().ok_or_else(|| missing_arg("kind"))?;
        brain.db.feedback(name, signal, kind).await?;
        Ok(json!({"ok": true}))
    }

    pub(in crate::mcp) async fn handle_explain_rank(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        let rank = brain.db.explain_rank(name).await?;
        Ok(serde_json::to_value(rank)?)
    }

    pub(in crate::mcp) async fn handle_sequential_thinking(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let step = args["step"].as_i64().ok_or_else(|| missing_arg("step"))?;
        let thought = args["thought"]
            .as_str()
            .ok_or_else(|| missing_arg("thought"))?;
        let next_step = args["next_step"]
            .as_i64()
            .ok_or_else(|| missing_arg("next_step"))?;
        let rowid: i64 = brain.db.add_thought(step, thought).await?;
        Ok(json!({"rowid": rowid, "step": step, "thought": thought, "next_step": next_step}))
    }

    pub(in crate::mcp) async fn handle_conclusion(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let summary = args["summary"]
            .as_str()
            .ok_or_else(|| missing_arg("summary"))?;
        let rowid: i64 = brain.db.add_conclusion(summary).await?;
        Ok(json!({"rowid": rowid, "summary": summary}))
    }

    pub(in crate::mcp) async fn handle_thought_history(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);
        let thoughts: Vec<(i64, i64, String, chrono::DateTime<chrono::Utc>)> =
            brain.db.get_thoughts(limit).await?;
        Ok(json!({"thoughts": thoughts}))
    }

    pub(in crate::mcp) async fn handle_thought_reset(&self, _args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let cleared: i64 = brain.db.clear_thoughts().await?;
        Ok(json!({"cleared": cleared}))
    }
}
