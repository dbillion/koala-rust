use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
        tool(
            "plugin_load",
            "Load a plugin",
            {
                let mut p = Map::new();
                p.insert("name".to_string(), json!({"type": "string"}));
                p
            },
            Some(vec!["name"]),
        ),
        tool(
            "plugin_call",
            "Call a plugin tool",
            {
                let mut p = Map::new();
                p.insert("plugin".to_string(), json!({"type": "string"}));
                p.insert("tool".to_string(), json!({"type": "string"}));
                p.insert("args".to_string(), json!({"type": "object"}));
                p
            },
            Some(vec!["plugin", "tool", "args"]),
        ),
        tool("plugin_list", "List loaded plugins", Map::new(), None),
        tool(
            "plugin_discover",
            "Discover and load plugins from directories",
            Map::new(),
            None,
        ),
    ]
}
