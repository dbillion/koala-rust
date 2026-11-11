//! Embeddings Module (v0.4) - Real Implementation using Model2Vec
//!
//! Provides local embedding generation using Model2Vec static token lookup models.
//! No neural network inference - just fast token embedding lookup + mean pooling.

use anyhow::Result;
use dashmap::DashMap;
use model2vec_rs::model::StaticModel;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Embedding model configuration
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub model_name: String,
    pub max_length: usize,
    pub batch_size: usize,
    pub normalize: bool,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            // Primary recommended model from Model2Vec maintainers
            model_name: "minishlab/potion-base-8M".to_string(),
            max_length: 512,
            batch_size: 1024,
            normalize: true,
        }
    }
}

/// Embedding dimension for potion-base-8M
pub const EMBEDDING_DIM: usize = 256;

/// Embedding manager with Model2Vec StaticModel and caching
pub struct EmbeddingManager {
    config: EmbeddingConfig,
    cache: DashMap<String, Vec<f32>>,
    model: Arc<Mutex<Option<StaticModel>>>,
}

impl EmbeddingManager {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            config,
            cache: DashMap::new(),
            model: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize the Model2Vec model (lazy loading from HF Hub or local path)
    pub async fn init(&self) -> Result<()> {
        let mut model_guard = self.model.lock().await;
        if model_guard.is_none() {
            info!("Loading Model2Vec model: {}", self.config.model_name);
            let model = StaticModel::from_pretrained(
                &self.config.model_name,
                None, // hf_token
                None, // normalize_embeddings
                None, // subfolder_in_repo
            )?;
            info!(
                "Model2Vec model loaded successfully (dim={})",
                EMBEDDING_DIM
            );
            *model_guard = Some(model);
        }
        Ok(())
    }

    /// Embed with caching
    pub async fn embed_cached(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first
        if let Some(cached) = self.cache.get(text) {
            return Ok(cached.clone());
        }

        // Generate embedding
        self.init().await?;
        let model_guard = self.model.lock().await;
        let model = model_guard.as_ref().unwrap();

        // Encode single text
        let texts = vec![text.to_string()];
        let embeddings = model.encode(&texts);
        let embedding = embeddings.into_iter().next().unwrap();

        // Cache it
        self.cache.insert(text.to_string(), embedding.clone());
        Ok(embedding)
    }

    /// Embed batch with caching
    pub async fn embed_batch_cached(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        let mut uncached_texts = Vec::new();
        let mut uncached_indices = Vec::new();

        // Check cache for each text
        for (i, text) in texts.iter().enumerate() {
            if let Some(cached) = self.cache.get(*text) {
                results.push(cached.clone());
            } else {
                uncached_texts.push(text.to_string());
                uncached_indices.push(i);
                results.push(Vec::new()); // placeholder
            }
        }

        // Generate embeddings for uncached texts
        if !uncached_texts.is_empty() {
            self.init().await?;
            let model_guard = self.model.lock().await;
            let model = model_guard.as_ref().unwrap();

            let embeddings = model.encode(&uncached_texts);

            // Fill in results and cache
            for (idx, embedding) in uncached_indices.into_iter().zip(embeddings) {
                self.cache
                    .insert(uncached_texts[idx].clone(), embedding.clone());
                results[idx] = embedding;
            }
        }

        Ok(results)
    }

    /// Embed without caching (for one-off use)
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.init().await?;
        let model_guard = self.model.lock().await;
        let model = model_guard.as_ref().unwrap();
        let texts = vec![text.to_string()];
        let embeddings = model.encode(&texts);
        Ok(embeddings.into_iter().next().unwrap())
    }

    /// Embed batch without caching
    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.init().await?;
        let model_guard = self.model.lock().await;
        let model = model_guard.as_ref().unwrap();
        let owned_texts: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
        Ok(model.encode(&owned_texts))
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache stats
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.config.max_length)
    }

    /// Get model info
    pub async fn model_info(&self) -> Result<String> {
        self.init().await?;
        Ok(format!(
            "Model2Vec: {} (dim={})",
            self.config.model_name, EMBEDDING_DIM
        ))
    }
}

/// RAG (Retrieval-Augmented Generation) pipeline
pub struct RagPipeline {
    embedding_manager: Arc<EmbeddingManager>,
    vector_engine: Arc<crate::vector::VectorEngine>,
}

impl RagPipeline {
    pub fn new(
        embedding_manager: Arc<EmbeddingManager>,
        vector_engine: Arc<crate::vector::VectorEngine>,
    ) -> Self {
        Self {
            embedding_manager,
            vector_engine,
        }
    }

    /// Add document to RAG index
    pub async fn add_document(
        &self,
        doc_id: i64,
        text: &str,
        _metadata: serde_json::Value,
    ) -> Result<()> {
        let embedding = self.embedding_manager.embed_cached(text).await?;
        self.vector_engine
            .upsert_fact_embedding(doc_id, &embedding)
            .await?;
        Ok(())
    }

    /// Search relevant documents
    pub async fn search(
        &self,
        query: &str,
        k: usize,
    ) -> Result<Vec<crate::vector::VectorSearchResult>> {
        let query_embedding = self.embedding_manager.embed_cached(query).await?;
        self.vector_engine
            .search_facts(&query_embedding, k, None)
            .await
    }

    /// Hybrid search: vector + keyword
    pub async fn hybrid_search(
        &self,
        query: &str,
        k: usize,
        keyword_filter: Option<&str>,
    ) -> Result<Vec<crate::vector::VectorSearchResult>> {
        let query_embedding = self.embedding_manager.embed_cached(query).await?;
        self.vector_engine
            .search_facts(&query_embedding, k, keyword_filter)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedding_manager() {
        let config = EmbeddingConfig::default();
        let manager = EmbeddingManager::new(config);

        let emb = manager.embed_cached("test").await.unwrap();
        assert_eq!(emb.len(), EMBEDDING_DIM);

        // Same text should give same embedding (deterministic + cached)
        let emb2 = manager.embed_cached("test").await.unwrap();
        assert_eq!(emb, emb2);

        // Different text should give different embedding
        let emb3 = manager.embed_cached("different").await.unwrap();
        assert_ne!(emb, emb3);
    }

    #[tokio::test]
    async fn test_embed_batch() {
        let config = EmbeddingConfig::default();
        let manager = EmbeddingManager::new(config);

        let texts = vec!["hello", "world", "test"];
        let embeddings = manager.embed_batch_cached(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        for emb in embeddings {
            assert_eq!(emb.len(), EMBEDDING_DIM);
        }
    }

    #[tokio::test]
    async fn test_semantic_similarity() {
        let config = EmbeddingConfig::default();
        let manager = EmbeddingManager::new(config);

        // Semantically similar phrases should have higher cosine similarity
        let emb1 = manager
            .embed_cached("how to configure vector search")
            .await
            .unwrap();
        let emb2 = manager
            .embed_cached("how do I set up vector search")
            .await
            .unwrap();
        let emb3 = manager
            .embed_cached("the cat sat on the mat")
            .await
            .unwrap();

        let sim_12 = cosine_similarity(&emb1, &emb2);
        let sim_13 = cosine_similarity(&emb1, &emb3);
        let sim_23 = cosine_similarity(&emb2, &emb3);

        // Similar meaning should score higher than unrelated
        assert!(
            sim_12 > sim_13,
            "semantically similar should score higher: {} vs {}",
            sim_12,
            sim_13
        );
        assert!(
            sim_12 > sim_23,
            "semantically similar should score higher: {} vs {}",
            sim_12,
            sim_23
        );

        info!(
            "Similarity similar: {}, unrelated1: {}, unrelated2: {}",
            sim_12, sim_13, sim_23
        );
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}
