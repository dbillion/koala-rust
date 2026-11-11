use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::{sync, PeaBrain};

pub struct PeaBrainServer {
    brain: Arc<Mutex<PeaBrain>>,
}

impl PeaBrainServer {
    pub fn new(brain: PeaBrain) -> Self {
        Self {
            brain: Arc::new(Mutex::new(brain)),
        }
    }

    async fn handle_request(&self, request: Value) -> Value {
        let id = request.get("id").cloned().unwrap_or(json!(null));
        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(json!({}));

        let result = match method {
            "initialize" => self.handle_initialize(params).await,
            "tools/list" => self.handle_list_tools().await,
            "tools/call" => self.handle_tool_call(params).await,
            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        };

        match result {
            Ok(result) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            }),
            Err(e) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": e.to_string()
                }
            }),
        }
    }

    async fn handle_initialize(&self, _params: Value) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "peabrain-rust", "version": "0.2.0" }
        }))
    }

    async fn handle_list_tools(&self) -> Result<Value> {
        use serde_json::Map;

        fn tool(
            name: &str,
            desc: &str,
            props: Map<String, Value>,
            required: Option<Vec<&str>>,
        ) -> Value {
            let mut schema = Map::new();
            schema.insert("type".to_string(), json!("object"));
            schema.insert("properties".to_string(), json!(props));
            if let Some(req) = required {
                schema.insert("required".to_string(), json!(req));
            }
            let mut t = Map::new();
            t.insert("name".to_string(), json!(name));
            t.insert("description".to_string(), json!(desc));
            t.insert("inputSchema".to_string(), json!(schema));
            json!(t)
        }

        let _props = Map::new();
        let tools = vec![
            // Core tools
            tool("health", "Database diagnostics", Map::new(), None),
            tool(
                "write_fact",
                "Write a fact",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p.insert("text".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["name", "text"]),
            ),
            tool(
                "read_fact",
                "Read a fact",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["name"]),
            ),
            tool("list_facts", "List facts", Map::new(), None),
            tool(
                "read_fact_versions",
                "Fact versions",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p.insert("limit".to_string(), json!({"type": "integer"}));
                    p
                },
                Some(vec!["name"]),
            ),
            tool(
                "delete_fact",
                "Delete fact",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["name"]),
            ),
            tool(
                "delete_fact_prefix",
                "Delete facts by prefix",
                {
                    let mut p = Map::new();
                    p.insert("prefix".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["prefix"]),
            ),
            tool(
                "create_entities",
                "Create entity",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p.insert("type".to_string(), json!({"type": "string"}));
                    p.insert("project".to_string(), json!({"type": "string"}));
                    p.insert(
                        "observations".to_string(),
                        json!({"type": "array", "items": {"type": "string"}}),
                    );
                    p
                },
                Some(vec!["name"]),
            ),
            tool(
                "add_observations",
                "Add observations",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p.insert(
                        "observations".to_string(),
                        json!({"type": "array", "items": {"type": "string"}}),
                    );
                    p
                },
                Some(vec!["name", "observations"]),
            ),
            tool(
                "open_nodes",
                "Open entities",
                {
                    let mut p = Map::new();
                    p.insert(
                        "names".to_string(),
                        json!({"type": "array", "items": {"type": "string"}}),
                    );
                    p
                },
                Some(vec!["names"]),
            ),
            tool(
                "search_nodes",
                "Search nodes",
                {
                    let mut p = Map::new();
                    p.insert("query".to_string(), json!({"type": "string"}));
                    p.insert("limit".to_string(), json!({"type": "integer"}));
                    p
                },
                Some(vec!["query"]),
            ),
            tool(
                "create_relations",
                "Create relation",
                {
                    let mut p = Map::new();
                    p.insert("from".to_string(), json!({"type": "string"}));
                    p.insert("to".to_string(), json!({"type": "string"}));
                    p.insert("type".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["from", "to", "type"]),
            ),
            tool(
                "create_task_or_note",
                "Create task",
                {
                    let mut p = Map::new();
                    p.insert("title".to_string(), json!({"type": "string"}));
                    p.insert("body".to_string(), json!({"type": "string"}));
                    p.insert("status".to_string(), json!({"type": "string"}));
                    p.insert("project".to_string(), json!({"type": "string"}));
                    p.insert("priority".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["title"]),
            ),
            tool(
                "query_tasks",
                "Query tasks",
                {
                    let mut p = Map::new();
                    p.insert("status".to_string(), json!({"type": "string"}));
                    p.insert("project".to_string(), json!({"type": "string"}));
                    p.insert("search".to_string(), json!({"type": "string"}));
                    p.insert("limit".to_string(), json!({"type": "integer"}));
                    p
                },
                None,
            ),
            tool(
                "update_task",
                "Update task",
                {
                    let mut p = Map::new();
                    p.insert("id".to_string(), json!({"type": "integer"}));
                    p.insert("title".to_string(), json!({"type": "string"}));
                    p.insert("body".to_string(), json!({"type": "string"}));
                    p.insert("status".to_string(), json!({"type": "string"}));
                    p.insert("project".to_string(), json!({"type": "string"}));
                    p.insert("priority".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["id"]),
            ),
            tool(
                "model_init",
                "Init model",
                {
                    let mut p = Map::new();
                    p.insert("hidden".to_string(), json!({"type": "integer"}));
                    p.insert("lr".to_string(), json!({"type": "number"}));
                    p.insert("activation".to_string(), json!({"type": "string"}));
                    p
                },
                None,
            ),
            tool(
                "model_train",
                "Train model",
                {
                    let mut p = Map::new();
                    p.insert(
                        "xs".to_string(),
                        json!({"type": "array", "items": {"type": "number"}}),
                    );
                    p.insert(
                        "ys".to_string(),
                        json!({"type": "array", "items": {"type": "number"}}),
                    );
                    p.insert(
                        "targets".to_string(),
                        json!({"type": "array", "items": {"type": "number"}}),
                    );
                    p.insert("lr".to_string(), json!({"type": "number"}));
                    p.insert("steps".to_string(), json!({"type": "integer"}));
                    p
                },
                Some(vec!["xs", "ys", "targets"]),
            ),
            tool(
                "model_predict",
                "Predict",
                {
                    let mut p = Map::new();
                    p.insert(
                        "xs".to_string(),
                        json!({"type": "array", "items": {"type": "number"}}),
                    );
                    p.insert(
                        "ys".to_string(),
                        json!({"type": "array", "items": {"type": "number"}}),
                    );
                    p
                },
                Some(vec!["xs", "ys"]),
            ),
            tool("model_status", "Model status", Map::new(), None),
            tool(
                "feedback",
                "Feedback",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p.insert("signal".to_string(), json!({"type": "string"}));
                    p.insert("kind".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["name", "signal", "kind"]),
            ),
            tool(
                "explain_rank",
                "Explain rank",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p.insert("kind".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["name"]),
            ),
            tool(
                "sequential_thinking",
                "Sequential thinking",
                {
                    let mut p = Map::new();
                    p.insert("step".to_string(), json!({"type": "integer"}));
                    p.insert("thought".to_string(), json!({"type": "string"}));
                    p.insert("next_step".to_string(), json!({"type": "integer"}));
                    p
                },
                Some(vec!["step", "thought", "next_step"]),
            ),
            tool(
                "conclusion",
                "Add conclusion",
                {
                    let mut p = Map::new();
                    p.insert("summary".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["summary"]),
            ),
            tool(
                "thought_history",
                "Get thought history",
                {
                    let mut p = Map::new();
                    p.insert("limit".to_string(), json!({"type": "integer"}));
                    p
                },
                None,
            ),
            tool("thought_reset", "Reset thought history", Map::new(), None),
            tool(
                "model_save",
                "Save model",
                {
                    let mut p = Map::new();
                    p.insert("step".to_string(), json!({"type": "integer"}));
                    p
                },
                Some(vec!["step"]),
            ),
            tool(
                "model_load",
                "Load model",
                {
                    let mut p = Map::new();
                    p.insert("step".to_string(), json!({"type": "integer"}));
                    p
                },
                Some(vec!["step"]),
            ),
            // v0.2: Vector Search tools
            tool(
                "vector_upsert_fact",
                "Upsert fact embedding",
                {
                    let mut p = Map::new();
                    p.insert("fact_id".to_string(), json!({"type": "integer"}));
                    p.insert(
                        "embedding".to_string(),
                        json!({"type": "array", "items": {"type": "number"}}),
                    );
                    p
                },
                Some(vec!["fact_id", "embedding"]),
            ),
            tool(
                "vector_upsert_entity",
                "Upsert entity embedding",
                {
                    let mut p = Map::new();
                    p.insert("entity_id".to_string(), json!({"type": "integer"}));
                    p.insert(
                        "embedding".to_string(),
                        json!({"type": "array", "items": {"type": "number"}}),
                    );
                    p
                },
                Some(vec!["entity_id", "embedding"]),
            ),
            tool(
                "vector_search_facts",
                "Search facts by vector similarity",
                {
                    let mut p = Map::new();
                    p.insert(
                        "query_embedding".to_string(),
                        json!({"type": "array", "items": {"type": "number"}}),
                    );
                    p.insert("k".to_string(), json!({"type": "integer"}));
                    p.insert("filter".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["query_embedding", "k"]),
            ),
            tool(
                "vector_search_entities",
                "Search entities by vector similarity",
                {
                    let mut p = Map::new();
                    p.insert(
                        "query_embedding".to_string(),
                        json!({"type": "array", "items": {"type": "number"}}),
                    );
                    p.insert("k".to_string(), json!({"type": "integer"}));
                    p
                },
                Some(vec!["query_embedding", "k"]),
            ),
            tool(
                "vector_delete_fact",
                "Delete fact embedding",
                {
                    let mut p = Map::new();
                    p.insert("fact_id".to_string(), json!({"type": "integer"}));
                    p
                },
                Some(vec!["fact_id"]),
            ),
            tool(
                "vector_delete_entity",
                "Delete entity embedding",
                {
                    let mut p = Map::new();
                    p.insert("entity_id".to_string(), json!({"type": "integer"}));
                    p
                },
                Some(vec!["entity_id"]),
            ),
            // v0.3: Sync tools
            tool(
                "sync_write_fact",
                "Write fact with CRDT sync",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p.insert("text".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["name", "text"]),
            ),
            tool(
                "sync_read_fact",
                "Read fact from sync",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["name"]),
            ),
            tool("sync_list_facts", "List facts from sync", Map::new(), None),
            tool(
                "sync_create_entity",
                "Create entity with CRDT sync",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p.insert("type".to_string(), json!({"type": "string"}));
                    p.insert("project".to_string(), json!({"type": "string"}));
                    p.insert(
                        "observations".to_string(),
                        json!({"type": "array", "items": {"type": "string"}}),
                    );
                    p
                },
                Some(vec!["name"]),
            ),
            tool(
                "sync_get_changes",
                "Get pending sync changes",
                {
                    let mut p = Map::new();
                    p.insert("doc_type".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["doc_type"]),
            ),
            tool(
                "sync_merge_changes",
                "Merge changes from peer",
                {
                    let mut p = Map::new();
                    p.insert("doc_type".to_string(), json!({"type": "string"}));
                    p.insert(
                        "changes".to_string(),
                        json!({"type": "array", "items": {"type": "string"}}),
                    );
                    p
                },
                Some(vec!["doc_type", "changes"]),
            ),
            // v0.4: Embedding tools
            tool(
                "embed_text",
                "Generate embedding for text",
                {
                    let mut p = Map::new();
                    p.insert("text".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["text"]),
            ),
            tool(
                "embed_batch",
                "Generate embeddings for multiple texts",
                {
                    let mut p = Map::new();
                    p.insert(
                        "texts".to_string(),
                        json!({"type": "array", "items": {"type": "string"}}),
                    );
                    p
                },
                Some(vec!["texts"]),
            ),
            // v0.5: Plugin tools
            tool(
                "plugin_load",
                "Load a plugin",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["name"]),
            ),
            tool(
                "plugin_call",
                "Call a plugin tool",
                {
                    let mut p = Map::new();
                    p.insert("plugin".to_string(), json!({"type": "string"}));
                    p.insert("tool".to_string(), json!({"type": "string"}));
                    p.insert("args".to_string(), json!({"type": "object"}));
                    p
                },
                Some(vec!["plugin", "tool", "args"]),
            ),
            tool("plugin_list", "List loaded plugins", Map::new(), None),
            tool(
                "plugin_discover",
                "Discover and load plugins from directories",
                Map::new(),
                None,
            ),
            // v0.5: Plugin marketplace tools
            tool(
                "plugin_search",
                "Search plugins in registry",
                {
                    let mut p = Map::new();
                    p.insert("query".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["query"]),
            ),
            tool(
                "plugin_install",
                "Install a plugin from registry",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p.insert("version".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["name"]),
            ),
            tool(
                "plugin_uninstall",
                "Uninstall a plugin",
                {
                    let mut p = Map::new();
                    p.insert("name".to_string(), json!({"type": "string"}));
                    p
                },
                Some(vec!["name"]),
            ),
            tool(
                "plugin_registry_stats",
                "Get registry statistics",
                Map::new(),
                None,
            ),
            tool(
                "plugin_update_registry",
                "Update registry from remote",
                {
                    let mut p = Map::new();
                    p.insert("url".to_string(), json!({"type": "string"}));
                    p
                },
                None,
            ),
        ];
        Ok(json!({"tools": tools}))
    }

    async fn handle_tool_call(&self, params: Value) -> Result<Value> {
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

            _ => return Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }?;

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&result)?
            }]
        }))
    }

    // Core handlers
    async fn handle_write_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        let text = args["text"].as_str().unwrap();
        let rowid = brain.db.write_fact(name, text).await?;
        Ok(json!({"rowid": rowid, "name": name}))
    }

    async fn handle_read_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        let fact = brain.db.read_fact(name).await?;
        Ok(fact
            .map(|(rowid, text)| json!({"rowid": rowid, "name": name, "body": text}))
            .unwrap_or_else(|| json!({"error": "not found"})))
    }

    async fn handle_list_facts(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let facts = brain.db.list_facts().await?;
        Ok(json!(facts))
    }

    async fn handle_read_fact_versions(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);
        let versions = brain.db.read_fact_versions(name, limit).await?;
        Ok(json!(versions))
    }

    async fn handle_delete_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        let deleted = brain.db.delete_fact(name).await?;
        Ok(json!({"deleted": deleted}))
    }

    async fn handle_delete_fact_prefix(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let prefix = args["prefix"].as_str().unwrap();
        let count = brain.db.delete_fact_prefix(prefix).await?;
        Ok(json!({"deleted": count}))
    }

    async fn handle_create_entities(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
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

    async fn handle_add_observations(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        let observations = args["observations"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<String>>();
        let added = brain.db.add_observations(name, &observations).await?;
        Ok(json!({"rowid": added, "name": name, "added": added}))
    }

    async fn handle_open_nodes(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let names = args["names"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<String>>();
        let nodes = brain.db.open_nodes(&names).await?;
        Ok(json!({"nodes": nodes}))
    }

    async fn handle_search_nodes(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let query = args["query"].as_str().unwrap();
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);
        let results = brain.db.search_nodes(query, limit).await?;
        Ok(json!({"results": results}))
    }

    async fn handle_create_relations(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let from = args["from"].as_str().unwrap();
        let to = args["to"].as_str().unwrap();
        let rel_type = args["type"].as_str().unwrap();
        let rowid = brain.db.create_relation(from, to, rel_type).await?;
        Ok(json!({"rowid": rowid, "from": from, "to": to, "type": rel_type}))
    }

    async fn handle_create_task(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let title = args["title"].as_str().unwrap();
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

    async fn handle_query_tasks(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let status = args.get("status").and_then(|v| v.as_str());
        let project = args.get("project").and_then(|v| v.as_str());
        let search = args.get("search").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);
        let tasks = brain.db.query_tasks(status, project, search, limit).await?;
        Ok(json!({"tasks": tasks}))
    }

    async fn handle_update_task(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let id = args["id"].as_i64().unwrap();
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

    async fn handle_model_init(&self, args: Value) -> Result<Value> {
        let mut brain = self.brain.lock().await;
        let hidden = args.get("hidden").and_then(|v| v.as_u64()).unwrap_or(16) as usize;
        let lr = args.get("lr").and_then(|v| v.as_f64()).unwrap_or(0.01) as f32;
        let activation = args
            .get("activation")
            .and_then(|v| v.as_str())
            .unwrap_or("leaky");
        brain.init_model(hidden, lr, activation).await?;
        Ok(json!({"ok": true, "hidden": hidden, "activation": activation, "step": 0}))
    }

    async fn handle_model_train(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let model = brain.model.lock().await;
        let mut model_guard = model;
        let model_ref = model_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("no model loaded (model_init or model_load first)"))?;

        let xs: Vec<f32> = args["xs"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect::<Vec<f32>>();
        let ys: Vec<f32> = args["ys"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect::<Vec<f32>>();
        let targets: Vec<f32> = args["targets"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect::<Vec<f32>>();
        let lr = args.get("lr").and_then(|v| v.as_f64()).map(|x| x as f32);
        let steps = args.get("steps").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

        if let Some(lr) = lr {
            model_ref.lr = lr;
        }

        let loss = model_ref.train_batch(&xs, &ys, &targets, steps)?;
        Ok(json!({"ok": true, "step": model_ref.step, "train_loss": loss}))
    }

    async fn handle_model_predict(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let model = brain.model.lock().await;
        let model_ref = model
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no model loaded (model_init or model_load first)"))?;

        let xs: Vec<f32> = args["xs"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect();
        let ys: Vec<f32> = args["ys"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect();
        let values = model_ref.predict(&xs, &ys);
        Ok(json!({"ok": true, "values": values}))
    }

    async fn handle_model_status(&self, _args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let model = brain.model.lock().await;
        let status = if let Some(model_ref) = model.as_ref() {
            serde_json::to_value(model_ref.status())?
        } else {
            json!({
                "initialized": false,
                "hidden": 0,
                "activation": "none",
                "step": 0,
                "last_loss": null,
                "allocated": 0,
                "committed": 0,
                "backend": "pure-rust",
                "gpu": "unavailable",
                "gpu_reason": "no CUDA; CPU reference is the supported backend",
                "rank": {
                    "total_entities": 0,
                    "total_relations": 0,
                    "total_observations": 0,
                    "total_facts": 0,
                    "total_tasks": 0
                }
            })
        };
        Ok(status)
    }

    async fn handle_feedback(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        let signal = args["signal"].as_str().unwrap();
        let kind = args["kind"].as_str().unwrap();
        brain.db.feedback(name, signal, kind).await?;
        Ok(json!({"ok": true}))
    }

    async fn handle_explain_rank(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        let rank = brain.db.explain_rank(name).await?;
        Ok(serde_json::to_value(rank)?)
    }

    async fn handle_health(&self, _args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let health = brain.db.health().await?;
        Ok(serde_json::to_value(health)?)
    }

    async fn handle_sequential_thinking(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let step = args["step"].as_i64().unwrap();
        let thought = args["thought"].as_str().unwrap();
        let next_step = args["next_step"].as_i64().unwrap();
        let rowid: i64 = brain.db.add_thought(step, thought).await?;
        Ok(json!({"rowid": rowid, "step": step, "thought": thought, "next_step": next_step}))
    }

    async fn handle_conclusion(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let summary = args["summary"].as_str().unwrap();
        let rowid: i64 = brain.db.add_conclusion(summary).await?;
        Ok(json!({"rowid": rowid, "summary": summary}))
    }

    async fn handle_thought_history(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(100);
        let thoughts: Vec<(i64, i64, String, chrono::DateTime<chrono::Utc>)> =
            brain.db.get_thoughts(limit).await?;
        Ok(json!({"thoughts": thoughts}))
    }

    async fn handle_thought_reset(&self, _args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let cleared: i64 = brain.db.clear_thoughts().await?;
        Ok(json!({"cleared": cleared}))
    }

    async fn handle_model_save(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let model = brain.model.lock().await;
        let model_ref = model
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no model loaded"))?;
        let step = args["step"].as_i64().unwrap();
        let state = model_ref.to_state();
        let state_json = serde_json::to_string(&state)?;
        brain.db.save_model_full(step, &state_json).await?;
        Ok(json!({"ok": true, "step": step}))
    }

    async fn handle_model_load(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let step = args["step"].as_i64().unwrap();
        let state_json: Option<String> = brain.db.load_model_full(step).await?;
        if let Some(json_str) = state_json {
            let state: crate::model::ModelState = serde_json::from_str(&json_str)?;
            Ok(json!({
                "ok": true,
                "step": step,
                "state": state,
                "note": "Model state retrieved. Use model_init with same hidden/activation to re-initialize."
            }))
        } else {
            Ok(json!({"ok": false, "error": "no model found at step"}))
        }
    }

    // v0.2: Vector Search handlers
    async fn handle_vector_upsert_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let fact_id = args["fact_id"].as_i64().unwrap();
        let embedding: Vec<f32> = args["embedding"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect();
        brain.vector_upsert_fact(fact_id, &embedding).await?;
        Ok(json!({"ok": true}))
    }

    async fn handle_vector_upsert_entity(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let entity_id = args["entity_id"].as_i64().unwrap();
        let embedding: Vec<f32> = args["embedding"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect();
        brain.vector_upsert_entity(entity_id, &embedding).await?;
        Ok(json!({"ok": true}))
    }

    async fn handle_vector_search_facts(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let query_embedding: Vec<f32> = args["query_embedding"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect::<Vec<f32>>();
        let k = args["k"].as_u64().unwrap() as usize;
        let filter = args.get("filter").and_then(|v| v.as_str());
        let results = brain
            .vector_search_facts(&query_embedding, k, filter)
            .await?;
        Ok(json!({"results": results}))
    }

    async fn handle_vector_search_entities(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let query_embedding: Vec<f32> = args["query_embedding"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect::<Vec<f32>>();
        let k = args["k"].as_u64().unwrap() as usize;
        let results = brain.vector_search_entities(&query_embedding, k).await?;
        Ok(json!({"results": results}))
    }

    async fn handle_vector_delete_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let fact_id = args["fact_id"].as_i64().unwrap();
        brain.vector_delete_fact(fact_id).await?;
        Ok(json!({"ok": true}))
    }

    async fn handle_vector_delete_entity(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let entity_id = args["entity_id"].as_i64().unwrap();
        brain.vector_delete_entity(entity_id).await?;
        Ok(json!({"ok": true}))
    }

    // v0.3: Sync handlers
    async fn handle_sync_write_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        let text = args["text"].as_str().unwrap();
        let id = brain.sync_write_fact(name, text).await?;
        Ok(json!({"ok": true, "id": id}))
    }

    async fn handle_sync_read_fact(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        let fact = brain.sync_read_fact(name).await?;
        Ok(fact
            .map(|(id, text)| json!({"id": id, "text": text}))
            .unwrap_or_else(|| json!({"error": "not found"})))
    }

    async fn handle_sync_list_facts(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let facts = brain.sync_list_facts().await?;
        Ok(json!(facts))
    }

    async fn handle_sync_create_entity(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
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

    async fn handle_sync_get_changes(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let doc_type_str = args["doc_type"].as_str().unwrap();
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
        let changes_b64: Vec<String> = changes.iter().map(|c| base64::encode(c)).collect();
        Ok(json!({"changes": changes_b64}))
    }

    async fn handle_sync_merge_changes(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let doc_type_str = args["doc_type"].as_str().unwrap();
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
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        let changes: Vec<Vec<u8>> = changes_b64
            .iter()
            .filter_map(|s| base64::decode(s).ok())
            .collect();
        brain.sync_merge_changes(doc_type, &changes).await?;
        Ok(json!({"ok": true}))
    }

    // v0.4: Embedding handlers
    async fn handle_embed_text(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let text = args["text"].as_str().unwrap();
        let embedding = brain.embed_text(text).await?;
        Ok(json!({"embedding": embedding}))
    }

    async fn handle_embed_batch(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let texts: Vec<&str> = args["texts"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        let embeddings = brain.embed_batch(&texts).await?;
        Ok(json!({"embeddings": embeddings}))
    }

    // v0.5: Plugin handlers
    async fn handle_plugin_load(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        brain.load_plugin(name).await?;
        Ok(json!({"ok": true}))
    }

    async fn handle_plugin_call(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let plugin = args["plugin"].as_str().unwrap();
        // Normalize: strip .wasm extension if present
        let plugin_name = plugin.strip_suffix(".wasm").unwrap_or(plugin);
        let tool = args["tool"].as_str().unwrap();
        let tool_args = args["args"].clone();
        let result = brain.call_plugin_tool(plugin_name, tool, tool_args).await?;
        Ok(result)
    }

    async fn handle_plugin_list(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let plugins = brain.list_plugins();
        Ok(json!({"plugins": plugins}))
    }

    async fn handle_plugin_discover(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let loaded = brain.discover_plugins().await?;
        Ok(json!({"loaded": loaded}))
    }

    // v0.5: Plugin marketplace handlers
    async fn handle_plugin_search(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let query = args["query"].as_str().unwrap_or("");
        let results = brain.plugin_registry.search(query).await?;
        Ok(json!({"plugins": results}))
    }

    async fn handle_plugin_install(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        let version = args["version"].as_str();
        let path = brain.plugin_registry.install(name, version).await?;
        // Also load into plugin manager with registry JSON for builtin plugins
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or(name);
        // Strip .wasm extension for registry lookup
        let plugin_name = filename.strip_suffix(".wasm").unwrap_or(filename);
        brain
            .plugin_manager
            .load_builtin_plugin(plugin_name, &brain.plugin_registry)
            .await?;
        Ok(json!({"ok": true, "path": path.display().to_string()}))
    }

    async fn handle_plugin_uninstall(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().unwrap();
        brain.plugin_registry.uninstall(name).await?;
        // Also unload from plugin manager
        brain.plugin_manager.unload_plugin(name).await;
        Ok(json!({"ok": true}))
    }

    async fn handle_plugin_registry_stats(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let stats = brain.plugin_registry.stats().await?;
        Ok(stats)
    }

    async fn handle_plugin_update_registry(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let url = args.get("url").and_then(|v| v.as_str());
        let count = brain.plugin_registry.update_registry(url).await?;
        Ok(json!({"updated": count}))
    }
}

pub async fn run_stdio(brain: PeaBrain) -> Result<()> {
    let server = Arc::new(PeaBrainServer {
        brain: Arc::new(Mutex::new(brain)),
    });
    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let request: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let response = server.handle_request(request).await;
        let response_str = serde_json::to_string(&response)?;
        stdout.write_all(response_str.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}
