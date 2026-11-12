pub mod db;
pub mod embeddings;
pub mod mcp;
pub mod model;
pub mod plugin_marketplace;
pub mod plugins;
pub mod sync;
pub mod vector;

use anyhow::Result;
use db::Database;
use embeddings::{EmbeddingConfig, EmbeddingManager};
use model::{ModelState, NeuralModel};
use plugin_marketplace::PluginRegistry;
use plugins::{PluginManager, WasmHost};
use std::path::PathBuf;
use std::sync::Arc;
use sync::SyncManager;
use tokio::sync::Mutex;
use vector::{VectorConfig, VectorEngine};

/// One result of `Koala::recall`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RecallHit {
    pub rowid: i64,
    pub name: String,
    pub text: String,
    pub distance: f32,
    pub keyword_overlap: f32,
    pub score: f32,
}

fn embed_input(name: &str, text: &str) -> String {
    format!("{}: {}", name.replace('_', " "), text)
}

fn tokens(s: &str) -> std::collections::HashSet<String> {
    const STOP: &[&str] = &[
        "the", "and", "for", "how", "what", "who", "does", "with", "that", "this", "are", "can",
        "you", "your", "from", "get", "has", "have", "into", "about",
    ];
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3 && !STOP.contains(t))
        .map(|t| t.to_string())
        .collect()
}

#[derive(Clone)]
pub struct Koala {
    pub db: Database,
    pub model: Arc<Mutex<Option<NeuralModel>>>,
    pub vector_engine: Arc<VectorEngine>,
    pub sync_manager: Arc<SyncManager>,
    pub embedding_manager: Arc<EmbeddingManager>,
    pub plugin_manager: Arc<PluginManager>,
    pub plugin_registry: Arc<PluginRegistry>,
}

impl Koala {
    pub async fn new(state_dir: PathBuf) -> Result<Self> {
        let db = Database::new(state_dir.join("koala.db").as_path()).await?;

        // Initialize vector engine
        let vector_config = VectorConfig::default();
        let vector_engine = Arc::new(VectorEngine::new(db.pool.clone(), vector_config).await?);

        // Initialize sync manager
        let sync_manager = Arc::new(SyncManager::new(None).await?);

        // Initialize embedding manager
        let embedding_config = EmbeddingConfig::default();
        let embedding_manager = Arc::new(EmbeddingManager::new(embedding_config));

        // Initialize plugin registry
        let plugin_registry = Arc::new(PluginRegistry::new(&state_dir).await?);

        // Initialize plugin manager
        let wasm_host = WasmHost::new()
            .with_fact_write(Arc::new(|_name, _text| {
                // This would need access to db - simplified for now
                Ok(0)
            }))
            .with_fact_read(Arc::new(|_name| Ok(None)))
            .with_entity_create(Arc::new(|_name, _type_, _project, _obs| Ok(0)))
            .with_task_create(Arc::new(|_title, _body, _status, _project, _priority| {
                Ok(0)
            }))
            .with_model_predict(Arc::new(|_xs, _ys| Ok(Vec::new())))
            .with_vector_search(Arc::new(|_query, _k| Ok(Vec::new())));

        let plugin_manager = Arc::new(PluginManager::new(
            wasm_host,
            state_dir.join("plugins/native"),
            state_dir.join("plugins/wasm"),
        )?);

        // Auto-discover and load installed plugins
        if let Err(e) = plugin_manager.discover_plugins().await {
            tracing::warn!("Failed to discover plugins on startup: {}", e);
        }

        let this = Self {
            db,
            model: Arc::new(Mutex::new(None)),
            vector_engine,
            sync_manager,
            embedding_manager,
            plugin_manager,
            plugin_registry,
        };

        // Old DB with a different vector dimension: tables were rebuilt, refill them.
        if this.vector_engine.was_rebuilt() {
            match this.reindex_facts().await {
                Ok(n) => tracing::info!("Re-embedded {} facts after dimension change", n),
                Err(e) => tracing::warn!(
                    "Could not re-embed facts at startup ({}). Run the reindex_facts tool later.",
                    e
                ),
            }
        }

        Ok(this)
    }

    /// Write a fact and embed it in one step. Replaces the vector of the
    /// previous version of the same name. Returns the new rowid.
    pub async fn remember_semantic(&self, name: &str, text: &str) -> Result<i64> {
        let old = self.db.current_fact_rowids(name).await?;
        let rowid = self.db.write_fact(name, text).await?;
        let emb = self.embed_text(&embed_input(name, text)).await?;
        self.vector_upsert_fact(rowid, &emb).await?;
        for r in old {
            if r != rowid {
                let _ = self.vector_delete_fact(r).await;
            }
        }
        Ok(rowid)
    }

    /// Embed every current fact (used after a model or dimension change).
    pub async fn reindex_facts(&self) -> Result<usize> {
        let facts = self.db.current_facts().await?;
        if facts.is_empty() {
            return Ok(0);
        }
        let inputs: Vec<String> = facts.iter().map(|(_, n, t)| embed_input(n, t)).collect();
        let refs: Vec<&str> = inputs.iter().map(|s| s.as_str()).collect();
        let embs = self.embed_batch(&refs).await?;
        for ((rowid, _, _), emb) in facts.iter().zip(embs.iter()) {
            self.vector_upsert_fact(*rowid, emb).await?;
        }
        Ok(facts.len())
    }

    /// Hybrid search: vector distance, nudged by keyword overlap so exact
    /// names and ids that embeddings miss still rank. Lower score is better.
    pub async fn recall(
        &self,
        query: &str,
        k: usize,
        keyword_weight: f32,
    ) -> Result<Vec<RecallHit>> {
        let qe = self.embed_text(query).await?;
        let pool = (k * 4).max(20);
        let cands = self.vector_search_facts(&qe, pool, None).await?;
        let qtokens = tokens(query);
        let mut hits = Vec::new();
        for c in cands {
            let Some((name, text, current)) = self.db.read_fact_by_rowid(c.id).await? else {
                continue;
            };
            if !current {
                continue;
            }
            let hay = tokens(&format!("{} {}", name.replace('_', " "), text));
            let overlap = if qtokens.is_empty() {
                0.0
            } else {
                qtokens.iter().filter(|t| hay.contains(*t)).count() as f32 / qtokens.len() as f32
            };
            hits.push(RecallHit {
                rowid: c.id,
                name,
                text,
                distance: c.score,
                keyword_overlap: overlap,
                score: c.score - keyword_weight * overlap,
            });
        }
        hits.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(k);
        Ok(hits)
    }

    pub async fn init_model(&mut self, hidden: usize, lr: f32, activation: &str) -> Result<()> {
        let mut model = NeuralModel::new(2, hidden, 1, lr, activation)?;
        model.init_weights();
        *self.model.lock().await = Some(model);
        Ok(())
    }

    pub async fn train_model(
        &mut self,
        xs: &[f32],
        ys: &[f32],
        targets: &[f32],
        _lr: f32,
        steps: u32,
    ) -> Result<f32> {
        let mut model_guard = self.model.lock().await;
        let model = model_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("no model loaded (model_init or model_load first)"))?;
        model.train_batch(xs, ys, targets, steps)
    }

    pub async fn predict_model(&self, xs: &[f32], ys: &[f32]) -> Result<Vec<f32>> {
        let model_guard = self.model.lock().await;
        let model = model_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no model loaded (model_init or model_load first)"))?;
        Ok(model.predict(xs, ys))
    }

    pub async fn model_status(&self) -> Result<model::ModelStatus> {
        let model_guard = self.model.lock().await;
        let status = if let Some(model) = model_guard.as_ref() {
            model.status()
        } else {
            model::ModelStatus {
                initialized: false,
                hidden: 0,
                activation: "none".to_string(),
                step: 0,
                last_loss: None,
                allocated: 0,
                committed: 0,
                backend: "pure-rust".to_string(),
                gpu: "unavailable".to_string(),
                gpu_reason: "no model loaded".to_string(),
                rank: model::RankInfo::default(),
            }
        };
        Ok(status)
    }

    pub async fn save_model(&self, step: i64) -> Result<()> {
        let model_guard = self.model.lock().await;
        let model = model_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no model loaded"))?;
        let state = model.to_state();
        let json = serde_json::to_string(&state)?;
        self.db.save_model_full(step, &json).await?;
        Ok(())
    }

    pub async fn load_model(&mut self, step: i64) -> Result<Option<NeuralModel>> {
        let state_json: Option<String> = self.db.load_model_full(step).await?;
        if let Some(json_str) = state_json {
            let state: ModelState = serde_json::from_str(&json_str)?;
            let model = NeuralModel::from_state(state)?;
            *self.model.lock().await = Some(model);
            Ok(None)
        } else {
            Ok(None)
        }
    }

    // v0.2: Vector search methods
    pub async fn vector_upsert_fact(&self, fact_id: i64, embedding: &[f32]) -> Result<()> {
        self.vector_engine
            .upsert_fact_embedding(fact_id, embedding)
            .await
    }

    pub async fn vector_upsert_entity(&self, entity_id: i64, embedding: &[f32]) -> Result<()> {
        self.vector_engine
            .upsert_entity_embedding(entity_id, embedding)
            .await
    }

    pub async fn vector_search_facts(
        &self,
        query_embedding: &[f32],
        k: usize,
        filter: Option<&str>,
    ) -> Result<Vec<vector::VectorSearchResult>> {
        self.vector_engine
            .search_facts(query_embedding, k, filter)
            .await
    }

    pub async fn vector_search_entities(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<vector::VectorSearchResult>> {
        self.vector_engine.search_entities(query_embedding, k).await
    }

    pub async fn vector_delete_fact(&self, fact_id: i64) -> Result<()> {
        self.vector_engine.delete_fact_embedding(fact_id).await
    }

    pub async fn vector_delete_entity(&self, entity_id: i64) -> Result<()> {
        self.vector_engine.delete_entity_embedding(entity_id).await
    }

    // v0.3: Sync methods
    pub async fn sync_write_fact(&self, name: &str, text: &str) -> Result<i64> {
        self.sync_manager.sync_write_fact(name, text).await
    }

    pub async fn sync_read_fact(&self, name: &str) -> Result<Option<(i64, String)>> {
        self.sync_manager.sync_read_fact(name).await
    }

    pub async fn sync_list_facts(&self) -> Result<Vec<String>> {
        self.sync_manager.sync_list_facts().await
    }

    pub async fn sync_create_entity(
        &self,
        name: &str,
        type_: Option<&str>,
        project: Option<&str>,
        observations: &[String],
    ) -> Result<i64> {
        self.sync_manager
            .sync_create_entity(name, type_, project, observations)
            .await
    }

    pub async fn sync_get_changes(&self, doc_type: sync::SyncDocType) -> Result<Vec<Vec<u8>>> {
        let entries = self.sync_manager.get_pending_changes(doc_type).await?;
        let serialized: Vec<Vec<u8>> = entries
            .iter()
            .filter_map(|e| serde_json::to_vec(e).ok())
            .collect();
        Ok(serialized)
    }

    pub async fn sync_merge_changes(
        &self,
        doc_type: sync::SyncDocType,
        changes: &[Vec<u8>],
    ) -> Result<()> {
        let entries: Vec<sync::SyncEntry> = changes
            .iter()
            .filter_map(|c| serde_json::from_slice(c).ok())
            .collect();
        self.sync_manager.merge_changes(doc_type, &entries).await
    }

    // v0.4: Embedding methods
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        self.embedding_manager.embed_cached(text).await
    }

    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.embedding_manager.embed_batch_cached(texts).await
    }

    // v0.5: Plugin methods
    pub async fn load_plugin(&self, name: &str) -> Result<()> {
        if name.ends_with(".wasm") {
            self.plugin_manager.load_wasm_plugin(name, None, None).await
        } else {
            self.plugin_manager.load_native_plugin(name).await
        }
    }

    pub async fn call_plugin_tool(
        &self,
        plugin: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.plugin_manager.call_tool(plugin, tool, args).await
    }

    pub fn list_plugins(&self) -> Vec<plugins::PluginManifest> {
        self.plugin_manager.list_plugins()
    }

    pub async fn discover_plugins(&self) -> Result<Vec<String>> {
        self.plugin_manager.discover_plugins().await
    }
}
