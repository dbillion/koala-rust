use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
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
    ]
}
