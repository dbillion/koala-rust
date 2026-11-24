use super::super::{missing_arg, KoalaServer};
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_remember_semantic(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("name is required"))?;
        let text = args["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("text is required"))?;
        let ttl_seconds = args.get("ttl_seconds").and_then(|v| v.as_i64());
        let rowid = if let Some(ttl) = ttl_seconds {
            brain
                .remember_semantic_with_ttl(name, text, Some(ttl))
                .await?
        } else {
            brain.remember_semantic(name, text).await?
        };
        Ok(json!({"rowid": rowid, "name": name, "embedded": true, "ttl_seconds": ttl_seconds}))
    }

    pub(in crate::mcp) async fn handle_recall(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("query is required"))?;
        let k = args.get("k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let w = args
            .get("keyword_weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.15) as f32;
        let w = if w.is_finite() {
            w.clamp(0.0, 1.0)
        } else {
            0.15
        };
        let hits = brain.recall(query, k, w).await?;
        Ok(json!({"results": hits}))
    }

    pub(in crate::mcp) async fn handle_sweep_expired_facts(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let purged = brain.sweep_expired_facts().await?;
        Ok(json!({"purged_rows": purged}))
    }

    pub(in crate::mcp) async fn handle_reindex_facts(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let n = brain.reindex_facts().await?;
        Ok(json!({"reindexed": n}))
    }

    pub(in crate::mcp) async fn handle_embed_text(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let text = args["text"].as_str().ok_or_else(|| missing_arg("text"))?;
        let embedding = brain.embed_text(text).await?;
        Ok(json!({"embedding": embedding}))
    }

    pub(in crate::mcp) async fn handle_embed_batch(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let texts: Vec<&str> = args["texts"]
            .as_array()
            .ok_or_else(|| missing_arg("texts"))?
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        let embeddings = brain.embed_batch(&texts).await?;
        Ok(json!({"embeddings": embeddings}))
    }
}
