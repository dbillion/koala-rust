use super::super::{missing_arg, KoalaServer};
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_plugin_search(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let query = args["query"].as_str().unwrap_or("");
        let results = brain.plugin_registry.search(query).await?;
        Ok(json!({"plugins": results}))
    }

    pub(in crate::mcp) async fn handle_plugin_install(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        let version = args["version"].as_str();
        let path = brain.plugin_registry.install(name, version).await?;
        // Also load into plugin manager with registry JSON for builtin plugins
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or(name);
        // Strip .wasm extension for registry lookup
        let plugin_name = filename.strip_suffix(".wasm").unwrap_or(filename);
        brain
            .plugin_manager
            .load_builtin_plugin(plugin_name, &brain.plugin_registry)
            .await?;
        Ok(json!({"ok": true, "path": path.display().to_string()}))
    }

    pub(in crate::mcp) async fn handle_plugin_uninstall(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let name = args["name"].as_str().ok_or_else(|| missing_arg("name"))?;
        brain.plugin_registry.uninstall(name).await?;
        // Also unload from plugin manager
        // Unload failure during uninstall is non-fatal; the registry entry is already gone.
        if let Err(e) = brain.plugin_manager.unload_plugin(name).await {
            tracing::warn!("Unload after uninstall failed: {}", e);
        }
        Ok(json!({"ok": true}))
    }

    pub(in crate::mcp) async fn handle_plugin_registry_stats(&self) -> Result<Value> {
        let brain = self.brain.lock().await;
        let stats = brain.plugin_registry.stats().await?;
        Ok(stats)
    }

    pub(in crate::mcp) async fn handle_plugin_update_registry(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let url = args.get("url").and_then(|v| v.as_str());
        let count = brain.plugin_registry.update_registry(url).await?;
        Ok(json!({"updated": count}))
    }
}
