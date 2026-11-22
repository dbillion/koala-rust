use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
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
    ]
}
