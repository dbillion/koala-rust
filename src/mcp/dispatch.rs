use super::KoalaServer;
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_tool_call(&self, params: Value) -> Result<Value> {
        let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        let result = match name {
            // Core tools
            "write_fact" => self.handle_write_fact(args).await,
            "read_fact" => self.handle_read_fact(args).await,
            "list_facts" => self.handle_list_facts().await,
            "read_fact_versions" => self.handle_read_fact_versions(args).await,
            "delete_fact" => self.handle_delete_fact(args).await,
            "delete_fact_prefix" => self.handle_delete_fact_prefix(args).await,
            "create_entities" => self.handle_create_entities(args).await,
            "add_observations" => self.handle_add_observations(args).await,
            "open_nodes" => self.handle_open_nodes(args).await,
            "search_nodes" => self.handle_search_nodes(args).await,
            "create_relations" => self.handle_create_relations(args).await,
            "create_task_or_note" => self.handle_create_task(args).await,
            "query_tasks" => self.handle_query_tasks(args).await,
            "update_task" => self.handle_update_task(args).await,
            "model_init" => self.handle_model_init(args).await,
            "model_train" => self.handle_model_train(args).await,
            "model_predict" => self.handle_model_predict(args).await,
            "model_status" => self.handle_model_status(args).await,
            "feedback" => self.handle_feedback(args).await,
            "explain_rank" => self.handle_explain_rank(args).await,
            "sequential_thinking" => self.handle_sequential_thinking(args).await,
            "conclusion" => self.handle_conclusion(args).await,
            "thought_history" => self.handle_thought_history(args).await,
            "thought_reset" => self.handle_thought_reset(args).await,
            "model_save" => self.handle_model_save(args).await,
            "model_load" => self.handle_model_load(args).await,
            "health" => self.handle_health(args).await,

            // v0.2: Vector Search
            "vector_upsert_fact" => self.handle_vector_upsert_fact(args).await,
            "vector_upsert_entity" => self.handle_vector_upsert_entity(args).await,
            "vector_search_facts" => self.handle_vector_search_facts(args).await,
            "vector_search_entities" => self.handle_vector_search_entities(args).await,
            "vector_delete_fact" => self.handle_vector_delete_fact(args).await,
            "vector_delete_entity" => self.handle_vector_delete_entity(args).await,

            // v0.3: Sync
            "sync_write_fact" => self.handle_sync_write_fact(args).await,
            "sync_read_fact" => self.handle_sync_read_fact(args).await,
            "sync_list_facts" => self.handle_sync_list_facts().await,
            "sync_create_entity" => self.handle_sync_create_entity(args).await,
            "sync_get_changes" => self.handle_sync_get_changes(args).await,
            "sync_merge_changes" => self.handle_sync_merge_changes(args).await,

            // v0.4: Embeddings
            "remember_semantic" => self.handle_remember_semantic(args).await,
            "recall" => self.handle_recall(args).await,
            "reindex_facts" => self.handle_reindex_facts().await,
            "embed_text" => self.handle_embed_text(args).await,
            "embed_batch" => self.handle_embed_batch(args).await,

            // v0.5: Plugins
            "plugin_load" => self.handle_plugin_load(args).await,
            "plugin_call" => self.handle_plugin_call(args).await,
            "plugin_list" => self.handle_plugin_list().await,
            "plugin_discover" => self.handle_plugin_discover().await,

            // v0.5: Plugin marketplace
            "plugin_search" => self.handle_plugin_search(args).await,
            "plugin_install" => self.handle_plugin_install(args).await,
            "plugin_uninstall" => self.handle_plugin_uninstall(args).await,
            "plugin_registry_stats" => self.handle_plugin_registry_stats().await,
            "plugin_update_registry" => self.handle_plugin_update_registry(args).await,

            // AI Continuity & Context Lifecycle
            "get_user_context" => self.handle_get_user_context().await,
            "get_project_context" => self.handle_get_project_context(args).await,
            "bootstrap_context" => self.handle_bootstrap_context(args).await,
            "record_decision" => self.handle_record_decision(args).await,
            "update_project_status" => self.handle_update_project_status(args).await,
            "create_handoff" => self.handle_create_handoff(args).await,
            "search_memory" => self.handle_search_memory(args).await,

            _ => return Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }?;

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&result)?
            }]
        }))
    }
}
