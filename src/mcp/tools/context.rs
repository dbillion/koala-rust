use super::schema::tool;
use serde_json::{json, Map, Value};

pub(super) fn specs() -> Vec<Value> {
    vec![
        tool(
            "get_user_context",
            "Retrieve current user working profile, preferences, and user facts",
            Map::new(),
            None,
        ),
        tool(
            "get_project_context",
            "Retrieve project architecture, decisions, latest handoff, and open tasks",
            {
                let mut p = Map::new();
                p.insert("project".to_string(), json!({"type": "string"}));
                p
            },
            Some(vec!["project"]),
        ),
        tool(
            "bootstrap_context",
            "Full session bootstrap combining user profile and project context",
            {
                let mut p = Map::new();
                p.insert("project".to_string(), json!({"type": "string"}));
                p
            },
            Some(vec!["project"]),
        ),
        tool(
            "record_decision",
            "Record an architectural or technical decision for a project",
            {
                let mut p = Map::new();
                p.insert("project".to_string(), json!({"type": "string"}));
                p.insert("decision".to_string(), json!({"type": "string"}));
                p.insert("rationale".to_string(), json!({"type": "string"}));
                p
            },
            Some(vec!["project", "decision"]),
        ),
        tool(
            "update_project_status",
            "Update project status, blockers, and next step",
            {
                let mut p = Map::new();
                p.insert("project".to_string(), json!({"type": "string"}));
                p.insert("status".to_string(), json!({"type": "string"}));
                p.insert(
                    "blockers".to_string(),
                    json!({"type": "array", "items": {"type": "string"}}),
                );
                p.insert("next_step".to_string(), json!({"type": "string"}));
                p
            },
            Some(vec!["project", "status"]),
        ),
        tool(
            "create_handoff",
            "Create and persist an immutable handoff checkpoint when concluding or switching",
            {
                let mut p = Map::new();
                p.insert("project".to_string(), json!({"type": "string"}));
                p.insert("summary".to_string(), json!({"type": "string"}));
                p.insert("next_step".to_string(), json!({"type": "string"}));
                p.insert(
                    "blockers".to_string(),
                    json!({"type": "array", "items": {"type": "string"}}),
                );
                p
            },
            Some(vec!["project", "summary", "next_step"]),
        ),
        tool(
            "search_memory",
            "Search memory across facts and handoffs with optional project filter",
            {
                let mut p = Map::new();
                p.insert("query".to_string(), json!({"type": "string"}));
                p.insert("project".to_string(), json!({"type": "string"}));
                p.insert("limit".to_string(), json!({"type": "integer"}));
                p
            },
            Some(vec!["query"]),
        ),
    ]
}
