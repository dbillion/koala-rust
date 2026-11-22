use super::super::{missing_arg, KoalaServer};
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_vector_upsert_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let fact_id = args["fact_id"]
            .as_i64()
            .ok_or_else(|| missing_arg("fact_id"))?;
        let embedding: Vec<f32> = args["embedding"]
            .as_array()
            .ok_or_else(|| missing_arg("embedding"))?
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect();
        brain.vector_upsert_fact(fact_id, &embedding).await?;
        Ok(json!({"ok": true}))
    }

    pub(in crate::mcp) async fn handle_vector_upsert_entity(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let entity_id = args["entity_id"]
            .as_i64()
            .ok_or_else(|| missing_arg("entity_id"))?;
        let embedding: Vec<f32> = args["embedding"]
            .as_array()
            .ok_or_else(|| missing_arg("embedding"))?
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect();
        brain.vector_upsert_entity(entity_id, &embedding).await?;
        Ok(json!({"ok": true}))
    }

    pub(in crate::mcp) async fn handle_vector_search_facts(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let query_embedding: Vec<f32> = args["query_embedding"]
            .as_array()
            .ok_or_else(|| missing_arg("query_embedding"))?
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect::<Vec<f32>>();
        let k = args["k"].as_u64().ok_or_else(|| missing_arg("k"))? as usize;
        let filter = args.get("filter").and_then(|v| v.as_str());
        let results = brain
            .vector_search_facts(&query_embedding, k, filter)
            .await?;
        Ok(json!({"results": results}))
    }

    pub(in crate::mcp) async fn handle_vector_search_entities(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let query_embedding: Vec<f32> = args["query_embedding"]
            .as_array()
            .ok_or_else(|| missing_arg("query_embedding"))?
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect::<Vec<f32>>();
        let k = args["k"].as_u64().ok_or_else(|| missing_arg("k"))? as usize;
        let results = brain.vector_search_entities(&query_embedding, k).await?;
        Ok(json!({"results": results}))
    }

    pub(in crate::mcp) async fn handle_vector_delete_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let fact_id = args["fact_id"]
            .as_i64()
            .ok_or_else(|| missing_arg("fact_id"))?;
        brain.vector_delete_fact(fact_id).await?;
        Ok(json!({"ok": true}))
    }

    pub(in crate::mcp) async fn handle_vector_delete_entity(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let entity_id = args["entity_id"]
            .as_i64()
            .ok_or_else(|| missing_arg("entity_id"))?;
        brain.vector_delete_entity(entity_id).await?;
        Ok(json!({"ok": true}))
    }
}
