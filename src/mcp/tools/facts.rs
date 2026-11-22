use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
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
    ]
}
