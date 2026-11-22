use serde_json::{json, Map, Value};

pub(super) fn tool(
    name: &str,
    desc: &str,
    props: Map<String, Value>,
    required: Option<Vec<&str>>,
) -> Value {
    let mut schema = Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert("properties".to_string(), json!(props));
    if let Some(req) = required {
        schema.insert("required".to_string(), json!(req));
    }
    let mut t = Map::new();
    t.insert("name".to_string(), json!(name));
    t.insert("description".to_string(), json!(desc));
    t.insert("inputSchema".to_string(), json!(schema));
    json!(t)
}
