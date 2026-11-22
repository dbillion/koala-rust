use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
        tool(
            "remember_semantic",
            "Write a fact and embed it for semantic search in one call",
            {
                let mut p = Map::new();
                p.insert("name".to_string(), json!({"type": "string"}));
                p.insert("text".to_string(), json!({"type": "string"}));
                p
            },
            Some(vec!["name", "text"]),
        ),
        tool(
            "recall",
            "Semantic + keyword search over facts. Returns name, text and score (lower is better)",
            {
                let mut p = Map::new();
                p.insert("query".to_string(), json!({"type": "string"}));
                p.insert("k".to_string(), json!({"type": "integer", "default": 5}));
                p.insert(
                    "keyword_weight".to_string(),
                    json!({"type": "number", "default": 0.15}),
                );
                p
            },
            Some(vec!["query"]),
        ),
        tool(
            "reindex_facts",
            "Re-embed all current facts (after changing embedding model)",
            Map::new(),
            None,
        ),
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
    ]
}
