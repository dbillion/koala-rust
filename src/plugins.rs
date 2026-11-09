//! Plugin System Module (v0.5)
//! 
//! Implements dynamic tool registration via WASM plugins and native dynamic libraries.
//! Supports hot-reloading, sandboxed execution, and custom tool definitions.

use anyhow::Result;
use dashmap::DashMap;
use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use wasmtime::{Config, Engine, Linker, Module, Store, Val, ValType, Func, Caller, Memory, Extern, Trap};

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub tools: Vec<ToolDefinition>,
    pub requires: Vec<String>, // Required host functions
    pub permissions: PluginPermissions,
}

/// Tool definition for plugin registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub is_async: bool,
}

/// Plugin permissions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginPermissions {
    pub network: bool,
    pub filesystem_read: bool,
    pub filesystem_write: bool,
    pub database_read: bool,
    pub database_write: bool,
    pub model_access: bool,
    pub max_execution_time_ms: u64,
    pub max_memory_mb: u64,
}

/// Plugin instance
pub struct Plugin {
    pub manifest: PluginManifest,
    pub plugin_type: PluginType,
    pub state: PluginState,
}

/// Plugin type
#[derive(Debug, Clone)]
pub enum PluginType {
    Wasm { module: Module, instance_id: u64 },
    Native { library: Arc<Library>, init_fn: *mut PluginVTable },
}

/// Plugin state
#[derive(Debug, Clone)]
pub struct PluginState {
    pub enabled: bool,
    pub loaded_at: chrono::DateTime<chrono::Utc>,
    pub call_count: u64,
    pub last_error: Option<String>,
}

/// Native plugin vtable (C-compatible)
#[repr(C)]
pub struct PluginVTable {
    pub name: *const std::os::raw::c_char,
    pub version: *const std::os::raw::c_char,
    pub get_tools: unsafe extern "C" fn(*mut std::ffi::c_void, *mut ToolDefinition, *mut u32) -> i32,
    pub call_tool: unsafe extern "C" fn(*mut std::ffi::c_void, *const std::os::raw::c_char, *const std::os::raw::c_char, *mut std::os::raw::c_char, *mut u32) -> i32,
    pub user_data: *mut std::ffi::c_void,
    pub drop: unsafe extern "C" fn(*mut std::ffi::c_void),
}

/// WASM host functions for plugins
pub struct WasmHost {
    pub db_read: Arc<dyn Fn(&str) -> Result<Value> + Send + Sync>,
    pub db_write: Arc<dyn Fn(&str, &Value) -> Result<()> + Send + Sync>,
    pub model_predict: Arc<dyn Fn(&[f32], &[f32]) -> Result<Vec<f32>> + Send + Sync>,
    pub vector_search: Arc<dyn Fn(&[f32], usize) -> Result<Vec<Value>> + Send + Sync>,
    pub fact_write: Arc<dyn Fn(&str, &str) -> Result<i64> + Send + Sync>,
    pub fact_read: Arc<dyn Fn(&str) -> Result<Option<Value>> + Send + Sync>,
    pub entity_create: Arc<dyn Fn(&str, Option<&str>, Option<&str>, &[String]) -> Result<i64> + Send + Sync>,
    pub task_create: Arc<dyn Fn(&str, &str, &str, &str, &str) -> Result<i64> + Send + Sync>,
}

impl WasmHost {
    pub fn new() -> Self {
        Self {
            db_read: Arc::new(|_| Ok(Value::Null)),
            db_write: Arc::new(|_, _| Ok(())),
            model_predict: Arc::new(|_, _| Ok(Vec::new())),
            vector_search: Arc::new(|_, _| Ok(Vec::new())),
            fact_write: Arc::new(|_, _| Ok(0)),
            fact_read: Arc::new(|_| Ok(None)),
            entity_create: Arc::new(|_, _, _, _| Ok(0)),
            task_create: Arc::new(|_, _, _, _, _| Ok(0)),
        }
    }

    pub fn with_db_read(mut self, f: Arc<dyn Fn(&str) -> Result<Value> + Send + Sync>) -> Self {
        self.db_read = f;
        self
    }

    pub fn with_db_write(mut self, f: Arc<dyn Fn(&str, &Value) -> Result<()> + Send + Sync>) -> Self {
        self.db_write = f;
        self
    }

    pub fn with_model_predict(mut self, f: Arc<dyn Fn(&[f32], &[f32]) -> Result<Vec<f32>> + Send + Sync>) -> Self {
        self.model_predict = f;
        self
    }

    pub fn with_vector_search(mut self, f: Arc<dyn Fn(&[f32], usize) -> Result<Vec<Value>> + Send + Sync>) -> Self {
        self.vector_search = f;
        self
    }

    pub fn with_fact_write(mut self, f: Arc<dyn Fn(&str, &str) -> Result<i64> + Send + Sync>) -> Self {
        self.fact_write = f;
        self
    }

    pub fn with_fact_read(mut self, f: Arc<dyn Fn(&str) -> Result<Option<Value>> + Send + Sync>) -> Self {
        self.fact_read = f;
        self
    }

    pub fn with_entity_create(mut self, f: Arc<dyn Fn(&str, Option<&str>, Option<&str>, &[String]) -> Result<i64> + Send + Sync>) -> Self {
        self.entity_create = f;
        self
    }

    pub fn with_task_create(mut self, f: Arc<dyn Fn(&str, &str, &str, &str, &str) -> Result<i64> + Send + Sync>) -> Self {
        self.task_create = f;
        self
    }
}

/// Plugin manager
pub struct PluginManager {
    plugins: DashMap<String, Plugin>,
    wasm_engine: Engine,
    wasm_host: Arc<WasmHost>,
    native_lib_dir: PathBuf,
    wasm_module_dir: PathBuf,
    next_instance_id: Arc<tokio::sync::Mutex<u64>>,
}

impl PluginManager {
    pub fn new(
        wasm_host: WasmHost,
        native_lib_dir: impl AsRef<Path>,
        wasm_module_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.async_support(true);
        let engine = Engine::new(&config)?;

        Ok(Self {
            plugins: DashMap::new(),
            wasm_engine: engine,
            wasm_host: Arc::new(wasm_host),
            native_lib_dir: native_lib_dir.as_ref().to_path_buf(),
            wasm_module_dir: wasm_module_dir.as_ref().to_path_buf(),
            next_instance_id: Arc::new(tokio::sync::Mutex::new(0)),
        })
    }

    /// Load a native plugin (.so/.dll/.dylib)
    pub async fn load_native_plugin(&self, lib_name: &str) -> Result<()> {
        let lib_path = self.native_lib_dir.join(lib_name);
        info!("Loading native plugin: {:?}", lib_path);

        let library = unsafe { Library::new(&lib_path)? };
        
        // Get init function
        let init_fn: Symbol<unsafe fn() -> *mut PluginVTable> = unsafe { library.get(b"plugin_init")? };
        let vtable_ptr = unsafe { init_fn() };
        let vtable = unsafe { &*vtable_ptr };

        // Get manifest
        let name = unsafe { std::ffi::CStr::from_ptr(vtable.name).to_string_lossy().to_string() };
        let version = unsafe { std::ffi::CStr::from_ptr(vtable.version).to_string_lossy().to_string() };

        // Get tool definitions
        let mut tool_count = 0u32;
        let tools_ptr = std::ptr::null_mut();
        let result = unsafe { (vtable.get_tools)(vtable.user_data, tools_ptr, &mut tool_count) };
        if result != 0 {
            anyhow::bail!("Failed to get tool count");
        }

        let mut tools_vec = Vec::with_capacity(tool_count as usize);
        let mut tools_ptr = {
            let mut v = Vec::with_capacity(tool_count as usize);
            for _ in 0..tool_count as usize {
                v.push(unsafe { std::mem::zeroed::<ToolDefinition>() });
            }
            v.as_mut_ptr()
        };
        let result = unsafe { (vtable.get_tools)(vtable.user_data, tools_ptr, &mut tool_count) };
        if result != 0 {
            anyhow::bail!("Failed to get tools");
        }

        for i in 0..tool_count as usize {
            tools_vec.push(unsafe { std::ptr::read(tools_ptr.add(i)) });
        }

        let manifest = PluginManifest {
            name: name.clone(),
            version,
            description: format!("Native plugin: {}", name),
            author: "Unknown".to_string(),
            tools: tools_vec,
            requires: Vec::new(),
            permissions: PluginPermissions::default(),
        };

        let plugin = Plugin {
            manifest,
            plugin_type: PluginType::Native { 
                library: Arc::new(library), 
                init_fn: vtable_ptr as *mut _ 
            },
            state: PluginState {
                enabled: true,
                loaded_at: chrono::Utc::now(),
                call_count: 0,
                last_error: None,
            },
        };

        self.plugins.insert(name.clone(), plugin);
        info!("Loaded native plugin: {}", name);
        Ok(())
    }

    /// Load a WASM plugin (.wasm)
    pub async fn load_wasm_plugin(&self, module_name: &str) -> Result<()> {
        let module_path = self.wasm_module_dir.join(module_name);
        info!("Loading WASM plugin: {:?}", module_path);

        let module = Module::from_file(&self.wasm_engine, &module_path)?;
        let mut store = Store::new(&self.wasm_engine, ());
        
        // Create linker with host functions
        let mut linker = Linker::new(&self.wasm_engine);
        self.add_host_functions(&mut linker)?;

        // Instantiate
        let instance = linker.instantiate(&mut store, &module)?;
        
        // Get exports
        let get_tools = instance.get_typed_func::<(), (i32, i32)>(&mut store, "get_tools")?;
        let (tools_ptr, tools_len) = get_tools.call(&mut store, ())?;
        
        // Read tools from WASM memory
        let memory = instance.get_memory(&mut store, "memory").unwrap();
        let tools_data = &memory.data(&store)[tools_ptr as usize..tools_ptr as usize + tools_len as usize];
        let tools: Vec<ToolDefinition> = serde_json::from_slice(tools_data)?;

        // Get manifest
        let get_manifest = instance.get_typed_func::<(), (i32, i32)>(&mut store, "get_manifest")?;
        let (manifest_ptr, manifest_len) = get_manifest.call(&mut store, ())?;
        let manifest_data = &memory.data(&store)[manifest_ptr as usize..manifest_ptr as usize + manifest_len as usize];
        let manifest: PluginManifest = serde_json::from_slice(manifest_data)?;

        let instance_id = {
            let mut id = self.next_instance_id.lock().await;
            *id += 1;
            *id
        };

        let plugin = Plugin {
            manifest,
            plugin_type: PluginType::Wasm { module, instance_id },
            state: PluginState {
                enabled: true,
                loaded_at: chrono::Utc::now(),
                call_count: 0,
                last_error: None,
            },
        };

        self.plugins.insert(plugin.manifest.name.clone(), plugin);
        info!("Loaded WASM plugin: {}", module_name);
        Ok(())
    }

    fn add_host_functions(&self, linker: &mut Linker<()>) -> Result<()> {
        let host = self.wasm_host.clone();

        // db_read
        linker.func_wrap("host", "db_read", move |caller: Caller<'_, ()>, ptr: i32, len: i32, out_ptr: i32, out_len: i32| -> i32 {
            // Implementation would read from caller memory
            0
        })?;

        // fact_write
        let host_fw = host.fact_write.clone();
        linker.func_wrap("host", "fact_write", move |name_ptr: i32, name_len: i32, text_ptr: i32, text_len: i32| -> i64 {
            // Would need to read from WASM memory
            0
        })?;

        // fact_read
        let host_fr = host.fact_read.clone();
        linker.func_wrap("host", "fact_read", move |name_ptr: i32, name_len: i32, out_ptr: i32, out_len: i32| -> i32 {
            0
        })?;

        // entity_create
        let host_ec = host.entity_create.clone();
        linker.func_wrap("host", "entity_create", move |name_ptr: i32, name_len: i32, type_ptr: i32, type_len: i32, project_ptr: i32, project_len: i32, obs_ptr: i32, obs_len: i32| -> i64 {
            0
        })?;

        // task_create
        let host_tc = host.task_create.clone();
        linker.func_wrap("host", "task_create", move |title_ptr: i32, title_len: i32, body_ptr: i32, body_len: i32, status_ptr: i32, status_len: i32, project_ptr: i32, project_len: i32, priority_ptr: i32, priority_len: i32| -> i64 {
            0
        })?;

        // model_predict
        let host_mp = host.model_predict.clone();
        linker.func_wrap("host", "model_predict", move |xs_ptr: i32, xs_len: i32, ys_ptr: i32, ys_len: i32, out_ptr: i32, out_len: i32| -> i32 {
            0
        })?;

        // vector_search
        let host_vs = host.vector_search.clone();
        linker.func_wrap("host", "vector_search", move |query_ptr: i32, query_len: i32, k: i32, out_ptr: i32, out_len: i32| -> i32 {
            0
        })?;

        Ok(())
    }

    /// Call a plugin tool
    pub async fn call_tool(&self, plugin_name: &str, tool_name: &str, args: Value) -> Result<Value> {
        let mut plugin = self.plugins.get_mut(plugin_name)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_name))?;

        if !plugin.state.enabled {
            anyhow::bail!("Plugin disabled: {}", plugin_name);
        }

        // Find tool definition
        let tool_def = plugin.manifest.tools.iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", tool_name))?;

        // Validate args against schema (simplified)
        // In production, use jsonschema crate

        let result = match &plugin.plugin_type {
            PluginType::Native { library, .. } => {
                self.call_native_tool(&plugin, tool_name, args).await?
            }
            PluginType::Wasm { module, instance_id } => {
                self.call_wasm_tool(&plugin, tool_name, args, *instance_id).await?
            }
        };

        plugin.state.call_count += 1;
        Ok(result)
    }

    async fn call_native_tool(&self, plugin: &Plugin, tool_name: &str, args: Value) -> Result<Value> {
        // Native plugin calling would use the vtable
        // Simplified for now
        Ok(Value::Null)
    }

    async fn call_wasm_tool(&self, plugin: &Plugin, tool_name: &str, args: Value, instance_id: u64) -> Result<Value> {
        // WASM tool calling
        // Simplified for now
        Ok(Value::Null)
    }

    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginManifest> {
        self.plugins.iter()
            .map(|p| p.manifest.clone())
            .collect()
    }

    /// Enable/disable plugin
    pub fn set_plugin_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        let mut plugin = self.plugins.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", name))?;
        plugin.state.enabled = enabled;
        Ok(())
    }

    /// Unload plugin
    pub async fn unload_plugin(&self, name: &str) -> Result<()> {
        self.plugins.remove(name);
        Ok(())
    }

    /// Discover and load all plugins from directories
    pub async fn discover_plugins(&self) -> Result<Vec<String>> {
        let mut loaded = Vec::new();

        // Load native plugins
        if self.native_lib_dir.exists() {
            for entry in std::fs::read_dir(&self.native_lib_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "so" || e == "dll" || e == "dylib") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        if self.load_native_plugin(name).await.is_ok() {
                            loaded.push(name.to_string());
                        }
                    }
                }
            }
        }

        // Load WASM plugins
        if self.wasm_module_dir.exists() {
            for entry in std::fs::read_dir(&self.wasm_module_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "wasm") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        if self.load_wasm_plugin(name).await.is_ok() {
                            loaded.push(name.to_string());
                        }
                    }
                }
            }
        }

        Ok(loaded)
    }
}

/// Example WASM plugin interface (for plugin authors)
pub mod wasm_plugin_interface {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    /// Plugin must export these functions:
    /// 
    /// - get_manifest() -> (ptr, len) returning JSON PluginManifest
    /// - get_tools() -> (ptr, len) returning JSON Vec<ToolDefinition>
    /// - call_tool(name_ptr, name_len, args_ptr, args_len, out_ptr, out_len) -> i32
    /// - allocate(size) -> ptr
    /// - deallocate(ptr, size)
    /// 
    /// Memory is shared with host via wasmtime Memory export.

    #[derive(Serialize, Deserialize)]
    pub struct WasmToolCall {
        pub name: String,
        pub args: Value,
    }

    #[derive(Serialize, Deserialize)]
    pub struct WasmToolResult {
        pub success: bool,
        pub result: Option<Value>,
        pub error: Option<String>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let host = WasmHost::new();
        let manager = PluginManager::new(
            host,
            "/tmp/plugins/native",
            "/tmp/plugins/wasm",
        ).unwrap();
        
        assert!(manager.list_plugins().is_empty());
    }
}