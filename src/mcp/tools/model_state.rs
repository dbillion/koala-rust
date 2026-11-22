use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
        tool(
            "model_save",
            "Save model",
            {
                let mut p = Map::new();
                p.insert("step".to_string(), json!({"type": "integer"}));
                p
            },
            Some(vec!["step"]),
        ),
        tool(
            "model_load",
            "Load model",
            {
                let mut p = Map::new();
                p.insert("step".to_string(), json!({"type": "integer"}));
                p
            },
            Some(vec!["step"]),
        ),
    ]
}
