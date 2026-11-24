use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
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
                p.insert(
                    "filter".to_string(),
                    json!({"type": "string", "description": "Literal keyword matched against fact names and text; not a SQL expression."}),
                );
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
    ]
}
