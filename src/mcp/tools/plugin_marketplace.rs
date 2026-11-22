use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
        tool(
            "plugin_search",
            "Search plugins in registry",
            {
                let mut p = Map::new();
                p.insert("query".to_string(), json!({"type": "string"}));
                p
            },
            Some(vec!["query"]),
        ),
        tool(
            "plugin_install",
            "Install a plugin from registry",
            {
                let mut p = Map::new();
                p.insert("name".to_string(), json!({"type": "string"}));
                p.insert("version".to_string(), json!({"type": "string"}));
                p
            },
            Some(vec!["name"]),
        ),
        tool(
            "plugin_uninstall",
            "Uninstall a plugin",
            {
                let mut p = Map::new();
                p.insert("name".to_string(), json!({"type": "string"}));
                p
            },
            Some(vec!["name"]),
        ),
        tool(
            "plugin_registry_stats",
            "Get registry statistics",
            Map::new(),
            None,
        ),
        tool(
            "plugin_update_registry",
            "Update registry from remote",
            {
                let mut p = Map::new();
                p.insert("url".to_string(), json!({"type": "string"}));
                p
            },
            None,
        ),
    ]
}
