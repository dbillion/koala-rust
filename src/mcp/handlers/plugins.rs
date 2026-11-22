use super::super::{missing_arg, KoalaServer};
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_plugin_load(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        brain.load_plugin(name).await?;
        Ok(json!({"ok": true}))
    }

    pub(in crate::mcp) async fn handle_plugin_call(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let plugin = args["plugin"]
            .as_str()
            .ok_or_else(|| missing_arg("plugin"))?;
        // Normalize: strip .wasm extension if present
        let plugin_name = plugin.strip_suffix(".wasm").unwrap_or(plugin);
        let tool = args["tool"].as_str().ok_or_else(|| missing_arg("tool"))?;
        let tool_args = args["args"].clone();
        let result = brain.call_plugin_tool(plugin_name, tool, tool_args).await?;
        Ok(result)
    }

    pub(in crate::mcp) async fn handle_plugin_list(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let plugins = brain.list_plugins();
        Ok(json!({"plugins": plugins}))
    }

    pub(in crate::mcp) async fn handle_plugin_discover(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let loaded = brain.discover_plugins().await?;
        Ok(json!({"loaded": loaded}))
    }
}
