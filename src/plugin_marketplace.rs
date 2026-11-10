//! Plugin marketplace for discovering, installing, and managing WASM/native plugins.

use crate::plugins::{PluginManager, PluginType, PluginManifest, ToolDefinition};
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// Generated at build time by build.rs
include!(concat!(env!("OUT_DIR"), "/wasm_plugins.rs"));

/// Registry entry for a plugin in the marketplace
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginRegistryEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub download_url: String, // "builtin://hash-tools" or "https://..."
    pub sha256: Option<String>,
    pub plugin_type: String, // "wasm" or "native"
}

/// Local plugin registry index
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct PluginIndex {
    plugins: HashMap<String, PluginRegistryEntry>,
}

impl PluginIndex {
    fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Plugin marketplace registry
pub struct PluginRegistry {
    index: Arc<RwLock<PluginIndex>>,
    index_path: PathBuf,
    install_dir: PathBuf,
    wasm_module_dir: PathBuf,
    native_module_dir: PathBuf,
    client: Client,
}

impl PluginRegistry {
    pub async fn new(data_dir: &Path) -> Result<Self> {
        let index_path = data_dir.join("plugin_index.json");
        let install_dir = data_dir.join("plugins");
        let wasm_module_dir = install_dir.join("wasm");
        let native_module_dir = install_dir.join("native");

        std::fs::create_dir_all(&wasm_module_dir)?;
        std::fs::create_dir_all(&native_module_dir)?;

        let mut index = PluginIndex::load(&index_path).unwrap_or_default();

        // Populate builtin plugins if registry is empty
        if index.plugins.is_empty() {
            Self::populate_builtin_plugins(&mut index);
            index.save(&index_path)?;
        }

        Ok(Self {
            index: Arc::new(RwLock::new(index)),
            index_path,
            install_dir,
            wasm_module_dir,
            native_module_dir,
            client: Client::new(),
        })
    }

    fn populate_builtin_plugins(index: &mut PluginIndex) {
        let builtins = vec![
            PluginRegistryEntry {
                name: "hash-tools".to_string(),
                version: "1.0.0".to_string(),
                description: "Cryptographic hash functions (MD5, SHA1, SHA256, SHA512)".to_string(),
                author: "peabrain".to_string(),
                download_url: "builtin://hash-tools".to_string(),
                sha256: None,
                plugin_type: "wasm".to_string(),
            },
            PluginRegistryEntry {
                name: "text-transform".to_string(),
                version: "1.0.0".to_string(),
                description: "Text transformation utilities (uppercase, lowercase, reverse, base64_encode, base64_decode)".to_string(),
                author: "peabrain".to_string(),
                download_url: "builtin://text-transform".to_string(),
                sha256: None,
                plugin_type: "wasm".to_string(),
            },
            PluginRegistryEntry {
                name: "json-tools".to_string(),
                version: "1.0.0".to_string(),
                description: "JSON manipulation tools (query, transform, validate)".to_string(),
                author: "peabrain".to_string(),
                download_url: "builtin://json-tools".to_string(),
                sha256: None,
                plugin_type: "wasm".to_string(),
            },
        ];

        for plugin in builtins {
            index.plugins.insert(plugin.name.clone(), plugin);
        }
    }

    /// Search for plugins in the local registry
    pub async fn search(&self, query: &str) -> Result<Vec<PluginRegistryEntry>> {
        let index = self.index.read().await;
        let query_lower = query.to_lowercase();
        let results: Vec<PluginRegistryEntry> = index
            .plugins
            .values()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower)
                    || p.description.to_lowercase().contains(&query_lower)
                    || p.author.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect();
        Ok(results)
    }

    /// List all plugins in the registry
    pub async fn list(&self) -> Result<Vec<PluginRegistryEntry>> {
        let index = self.index.read().await;
        Ok(index.plugins.values().cloned().collect())
    }

    /// Get a specific plugin entry
    pub async fn get(&self, name: &str) -> Result<Option<PluginRegistryEntry>> {
        let index = self.index.read().await;
        Ok(index.plugins.get(name).cloned())
    }

    /// Install a plugin from the registry
    pub async fn install(&self, name: &str, version: Option<&str>) -> Result<PathBuf> {
        let plugin = {
            let index = self.index.read().await;
            index.plugins.get(name).cloned()
                .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found in registry", name))?
        };

        // Check if already installed
        let wasm_name = plugin.download_url.strip_prefix("builtin://").unwrap_or(&plugin.name);
        let wasm_filename = if wasm_name.ends_with(".wasm") {
            wasm_name.to_string()
        } else {
            format!("{}.wasm", wasm_name)
        };
        let install_path = self.wasm_module_dir.join(&wasm_filename);

        if install_path.exists() {
            info!("Plugin '{}' already installed at {:?}", name, install_path);
            return Ok(install_path);
        }

        // Download or generate the plugin
        let wasm_bytes = if plugin.download_url.starts_with("builtin://") {
            self.generate_builtin_wasm(&plugin.name).await?
        } else {
            self.download_wasm(&plugin.download_url).await?
        };

        // Verify SHA256 if provided
        if let Some(expected_sha256) = &plugin.sha256 {
            let actual_sha256 = self.compute_sha256(&wasm_bytes);
            if actual_sha256 != *expected_sha256 {
                return Err(anyhow::anyhow!(
                    "SHA256 mismatch for plugin '{}': expected {}, got {}",
                    name, expected_sha256, actual_sha256
                ));
            }
        }

        // Write the WASM file
        std::fs::write(&install_path, wasm_bytes)?;
        info!("Installed plugin '{}' to {:?}", name, install_path);

        Ok(install_path)
    }

    /// Uninstall a plugin
    pub async fn uninstall(&self, name: &str) -> Result<()> {
        let wasm_filename = format!("{}.wasm", name);
        let install_path = self.wasm_module_dir.join(&wasm_filename);
        
        if install_path.exists() {
            std::fs::remove_file(&install_path)?;
            info!("Uninstalled plugin '{}'", name);
        } else {
            warn!("Plugin '{}' not found for uninstall", name);
        }
        Ok(())
    }

    /// Update the registry from a remote source
    pub async fn update_registry(&self, registry_url: Option<&str>) -> Result<usize> {
        let url = registry_url.unwrap_or("https://raw.githubusercontent.com/dbillion/peabrain-plugins/main/registry.json");
        
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch registry: {}", response.status()));
        }
        
        let new_plugins: Vec<PluginRegistryEntry> = response.json().await?;
        let count = new_plugins.len();
        
        let mut index = self.index.write().await;
        for plugin in new_plugins {
            index.plugins.insert(plugin.name.clone(), plugin);
        }
        index.save(&self.index_path)?;
        
        info!("Updated registry with {} plugins", count);
        Ok(count)
    }

    /// Get registry stats
    pub async fn stats(&self) -> Result<serde_json::Value> {
        let index = self.index.read().await;
        let installed = std::fs::read_dir(&self.wasm_module_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "wasm"))
            .count();
        
        Ok(serde_json::json!({
            "total_plugins": index.plugins.len(),
            "installed_plugins": installed,
            "categories": self.categorize_plugins(&index).await,
        }))
    }

    async fn categorize_plugins(&self, index: &PluginIndex) -> HashMap<String, usize> {
        let mut categories = HashMap::new();
        for plugin in index.plugins.values() {
            *categories.entry(plugin.plugin_type.clone()).or_insert(0) += 1;
        }
        categories
    }

    /// Download a WASM plugin from a URL
    async fn download_wasm(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to download plugin: {}", response.status()));
        }
        Ok(response.bytes().await?.to_vec())
    }

    /// Generate a builtin WASM plugin (use pre-compiled WASM from build time)
        async fn generate_builtin_wasm(&self, name: &str) -> Result<Vec<u8>> {
            let wasm_bytes = match name {
                "hash-tools" => HASH_TOOLS_WASM,
                "text-transform" => TEXT_TRANSFORM_WASM,
                "json-tools" => JSON_TOOLS_WASM,
                _ => return Err(anyhow::anyhow!("Unknown builtin plugin: {}", name)),
            };

            Ok(wasm_bytes.to_vec())
        }

        /// Get builtin plugin manifest JSON
            pub fn get_builtin_manifest(&self, name: &str) -> serde_json::Value {
            match name {
                "hash-tools" => json!({
                    "name": "hash-tools",
                    "version": "1.0.0",
                    "description": "Cryptographic hash functions (MD5, SHA1, SHA256, SHA512)",
                    "author": "peabrain",
                    "tools": [{
                        "name": "hash",
                        "description": "Compute hash of input text",
                        "input_schema": {
                            "type": "object",
                            "properties": {
                                "algorithm": {"type": "string", "enum": ["md5", "sha1", "sha256", "sha512"]},
                                "text": {"type": "string"}
                            },
                            "required": ["algorithm", "text"]
                        },
                        "output_schema": {
                            "type": "object",
                            "properties": {"hash": {"type": "string"}},
                            "required": ["hash"]
                        },
                        "is_async": false
                    }],
                    "requires": [],
                    "permissions": {
                        "network": false, "filesystem_read": false, "filesystem_write": false,
                        "database_read": false, "database_write": false, "model_access": false,
                        "max_execution_time_ms": 5000, "max_memory_mb": 10
                    }
                }),
                "text-transform" => json!({
                    "name": "text-transform",
                    "version": "1.0.0",
                    "description": "Text transformation utilities (uppercase, lowercase, reverse, base64_encode, base64_decode)",
                    "author": "peabrain",
                    "tools": [{
                        "name": "transform",
                        "description": "Transform text (uppercase, lowercase, reverse, base64_encode, base64_decode)",
                        "input_schema": {
                            "type": "object",
                            "properties": {
                                "operation": {"type": "string", "enum": ["uppercase", "lowercase", "reverse", "base64_encode", "base64_decode"]},
                                "text": {"type": "string"}
                            },
                            "required": ["operation", "text"]
                        },
                        "output_schema": {
                            "type": "object",
                            "properties": {"result": {"type": "string"}},
                            "required": ["result"]
                        },
                        "is_async": false
                    }],
                    "requires": [],
                    "permissions": {
                        "network": false, "filesystem_read": false, "filesystem_write": false,
                        "database_read": false, "database_write": false, "model_access": false,
                        "max_execution_time_ms": 5000, "max_memory_mb": 10
                    }
                }),
                "json-tools" => json!({
                    "name": "json-tools",
                    "version": "1.0.0",
                    "description": "JSON manipulation tools (query, transform, validate)",
                    "author": "peabrain",
                    "tools": [{
                        "name": "json_query",
                        "description": "Query JSON using JSONPath-like syntax",
                        "input_schema": {
                            "type": "object",
                            "properties": {
                                "json": {"type": "string"},
                                "path": {"type": "string"}
                            },
                            "required": ["json", "path"]
                        },
                        "output_schema": {
                            "type": "object",
                            "properties": {"result": {}},
                            "required": ["result"]
                        },
                        "is_async": false
                    }, {
                        "name": "json_pretty",
                        "description": "Pretty-print JSON with indentation",
                        "input_schema": {
                            "type": "object",
                            "properties": {
                                "json": {"type": "string"},
                                "indent": {"type": "integer", "default": 2}
                            },
                            "required": ["json"]
                        },
                        "output_schema": {
                            "type": "object",
                            "properties": {"result": {"type": "string"}},
                            "required": ["result"]
                        },
                        "is_async": false
                    }],
                    "requires": [],
                    "permissions": {
                        "network": false, "filesystem_read": false, "filesystem_write": false,
                        "database_read": false, "database_write": false, "model_access": false,
                        "max_execution_time_ms": 5000, "max_memory_mb": 10
                    }
                }),
                _ => json!({}),
            }
        }

        /// Get builtin plugin tools JSON
            pub fn get_builtin_tools(&self, name: &str) -> serde_json::Value {
            match name {
                "hash-tools" => json!([{
                    "name": "hash",
                    "description": "Compute hash of input text",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "algorithm": {"type": "string", "enum": ["md5", "sha1", "sha256", "sha512"]},
                            "text": {"type": "string"}
                        },
                        "required": ["algorithm", "text"]
                    },
                    "output_schema": {
                        "type": "object",
                        "properties": {"hash": {"type": "string"}},
                        "required": ["hash"]
                    },
                    "is_async": false
                }]),
                "text-transform" => json!([{
                    "name": "transform",
                    "description": "Transform text (uppercase, lowercase, reverse, base64_encode, base64_decode)",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "operation": {"type": "string", "enum": ["uppercase", "lowercase", "reverse", "base64_encode", "base64_decode"]},
                            "text": {"type": "string"}
                        },
                        "required": ["operation", "text"]
                    },
                    "output_schema": {
                        "type": "object",
                        "properties": {"result": {"type": "string"}},
                        "required": ["result"]
                    },
                    "is_async": false
                }]),
                "json-tools" => json!([{
                    "name": "json_query",
                    "description": "Query JSON using JSONPath-like syntax",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "json": {"type": "string"},
                            "path": {"type": "string"}
                        },
                        "required": ["json", "path"]
                    },
                    "output_schema": {
                        "type": "object",
                        "properties": {"result": {}},
                        "required": ["result"]
                    },
                    "is_async": false
                }, {
                    "name": "json_pretty",
                    "description": "Pretty-print JSON with indentation",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "json": {"type": "string"},
                            "indent": {"type": "integer", "default": 2}
                        },
                        "required": ["json"]
                    },
                    "output_schema": {
                        "type": "object",
                        "properties": {"result": {"type": "string"}},
                        "required": ["result"]
                    },
                    "is_async": false
                }]),
                _ => json!([]),
            }
        }

    /// Compute SHA256 hash
    fn compute_sha256(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Get install directory
    pub fn wasm_module_dir(&self) -> &Path {
        &self.wasm_module_dir
    }

    /// Get native module directory
    pub fn native_module_dir(&self) -> &Path {
        &self.native_module_dir
    }
}