use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
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
    ]
}
