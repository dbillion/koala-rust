use super::schema::tool;
use serde_json::{Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![tool("health", "Database diagnostics", Map::new(), None)]
}
