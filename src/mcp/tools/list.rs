use super::super::KoalaServer;
use super::context;
use super::core;
use super::entities;
use super::facts;
use super::model;
use super::model_state;
use super::plugin_marketplace;
use super::plugins;
use super::recall;
use super::sync;
use super::tasks;
use super::thinking;
use super::vector;
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_list_tools(&self) -> Result<Value> {
        let mut tools = Vec::new();
        tools.extend(core::specs());
        tools.extend(facts::specs());
        tools.extend(entities::specs());
        tools.extend(tasks::specs());
        tools.extend(model::specs());
        tools.extend(thinking::specs());
        tools.extend(model_state::specs());
        tools.extend(vector::specs());
        tools.extend(sync::specs());
        tools.extend(recall::specs());
        tools.extend(plugins::specs());
        tools.extend(plugin_marketplace::specs());
        tools.extend(context::specs());
        Ok(json!({"tools": tools}))
    }
}
