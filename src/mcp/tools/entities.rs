use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
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
    ]
}
