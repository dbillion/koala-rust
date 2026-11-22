use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
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
                p.insert("doc_type".to_string(), json!({"type": "string", "enum": ["facts", "entities", "relations", "tasks", "thoughts", "model"]}));
                p
            },
            Some(vec!["doc_type"]),
        ),
        tool(
            "sync_merge_changes",
            "Merge changes from peer",
            {
                let mut p = Map::new();
                p.insert("doc_type".to_string(), json!({"type": "string", "enum": ["facts", "entities", "relations", "tasks", "thoughts", "model"]}));
                p.insert(
                    "changes".to_string(),
                    json!({"type": "array", "items": {"type": "string"}}),
                );
                p
            },
            Some(vec!["doc_type", "changes"]),
        ),
    ]
}
