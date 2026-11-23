//! Plugin System Module (v0.5)
//!
//! Implements dynamic tool registration via WASM plugins and native dynamic libraries.
//! Supports hot-reloading, sandboxed execution, and custom tool definitions.

use anyhow::Result;
use dashmap::DashMap;
#[cfg(feature = "native-plugins")]
use libloading::{Library, Symbol};
use md5::Md5;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;
use wasmtime::{Caller, Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

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

/// Opaque pointer to a native plugin's vtable (from the plugin's `plugin_init`
/// symbol). The vtable is a static structure inside the loaded `Library`
/// (which is Send + Sync); the pointer is only dereferenced while the plugin
/// manager holds its lock, so sharing it across threads is sound. Wrapping it
/// makes the `!Send`/`!Sync` raw pointer type explicit.
#[cfg(feature = "native-plugins")]
#[derive(Debug, Clone, Copy)]
pub struct PluginVTablePtr(pub *mut PluginVTable);

// SAFETY: the pointee is a static vtable inside the loaded library and
// outlives the plugin; all dereferences are serialized by PluginManager's lock.
#[cfg(feature = "native-plugins")]
unsafe impl Send for PluginVTablePtr {}
#[cfg(feature = "native-plugins")]
unsafe impl Sync for PluginVTablePtr {}

/// Plugin type
#[derive(Debug, Clone)]
pub enum PluginType {
    Wasm {
        module: Module,
        instance_id: u64,
    },
    #[cfg(feature = "native-plugins")]
    Native {
        library: Arc<Library>,
        init_fn: PluginVTablePtr,
    },
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
    pub get_tools:
        unsafe extern "C" fn(*mut std::ffi::c_void, *mut ToolDefinition, *mut u32) -> i32,
    pub call_tool: unsafe extern "C" fn(
        *mut std::ffi::c_void,
        *const std::os::raw::c_char,
        *const std::os::raw::c_char,
        *mut std::os::raw::c_char,
        *mut u32,
    ) -> i32,
    pub user_data: *mut std::ffi::c_void,
    pub drop: unsafe extern "C" fn(*mut std::ffi::c_void),
}

/// WASM host callback function types (extracted to keep `WasmHost` readable).
type DbReadFn = Arc<dyn Fn(&str) -> Result<Value> + Send + Sync>;
type DbWriteFn = Arc<dyn Fn(&str, &Value) -> Result<()> + Send + Sync>;
type ModelPredictFn = Arc<dyn Fn(&[f32], &[f32]) -> Result<Vec<f32>> + Send + Sync>;
type VectorSearchFn = Arc<dyn Fn(&[f32], usize) -> Result<Vec<Value>> + Send + Sync>;
type FactWriteFn = Arc<dyn Fn(&str, &str) -> Result<i64> + Send + Sync>;
type FactReadFn = Arc<dyn Fn(&str) -> Result<Option<Value>> + Send + Sync>;
type EntityCreateFn =
    Arc<dyn Fn(&str, Option<&str>, Option<&str>, &[String]) -> Result<i64> + Send + Sync>;
type TaskCreateFn = Arc<dyn Fn(&str, &str, &str, &str, &str) -> Result<i64> + Send + Sync>;

/// WASM host functions for plugins
pub struct WasmHost {
    pub db_read: DbReadFn,
    pub db_write: DbWriteFn,
    pub model_predict: ModelPredictFn,
    pub vector_search: VectorSearchFn,
    pub fact_write: FactWriteFn,
    pub fact_read: FactReadFn,
    pub entity_create: EntityCreateFn,
    pub task_create: TaskCreateFn,
}

impl Default for WasmHost {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn with_db_read(mut self, f: DbReadFn) -> Self {
        self.db_read = f;
        self
    }

    pub fn with_db_write(mut self, f: DbWriteFn) -> Self {
        self.db_write = f;
        self
    }

    pub fn with_model_predict(mut self, f: ModelPredictFn) -> Self {
        self.model_predict = f;
        self
    }

    pub fn with_vector_search(mut self, f: VectorSearchFn) -> Self {
        self.vector_search = f;
        self
    }

    pub fn with_fact_write(mut self, f: FactWriteFn) -> Self {
        self.fact_write = f;
        self
    }

    pub fn with_fact_read(mut self, f: FactReadFn) -> Self {
        self.fact_read = f;
        self
    }

    pub fn with_entity_create(mut self, f: EntityCreateFn) -> Self {
        self.entity_create = f;
        self
    }

    pub fn with_task_create(mut self, f: TaskCreateFn) -> Self {
        self.task_create = f;
        self
    }
}

/// Plugin manager
pub struct PluginManager {
    plugins: DashMap<String, Plugin>,
    wasm_engine: Engine,
    wasm_host: Arc<WasmHost>,
    #[cfg(feature = "native-plugins")]
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
        config.consume_fuel(true);
        let engine = Engine::new(&config)?;

        #[cfg(not(feature = "native-plugins"))]
        let _ = native_lib_dir;
        Ok(Self {
            plugins: DashMap::new(),
            wasm_engine: engine,
            wasm_host: Arc::new(wasm_host),
            #[cfg(feature = "native-plugins")]
            native_lib_dir: native_lib_dir.as_ref().to_path_buf(),
            wasm_module_dir: wasm_module_dir.as_ref().to_path_buf(),
            next_instance_id: Arc::new(tokio::sync::Mutex::new(0)),
        })
    }

    /// Load a native plugin (.so/.dll/.dylib). Native code is opt-in because it
    /// executes with the host process's full OS privileges.
    #[cfg(feature = "native-plugins")]
    pub async fn load_native_plugin(&self, lib_name: &str) -> Result<()> {
        let lib_path = self.native_lib_dir.join(lib_name);
        info!("Loading native plugin: {:?}", lib_path);

        let library = unsafe { Library::new(&lib_path)? };

        // Get init function
        let init_fn: Symbol<unsafe fn() -> *mut PluginVTable> =
            unsafe { library.get(b"plugin_init")? };
        let vtable_ptr = unsafe { init_fn() };
        let vtable = unsafe { &*vtable_ptr };

        // Get manifest
        let name = unsafe {
            std::ffi::CStr::from_ptr(vtable.name)
                .to_string_lossy()
                .to_string()
        };
        let version = unsafe {
            std::ffi::CStr::from_ptr(vtable.version)
                .to_string_lossy()
                .to_string()
        };

        // Get tool definitions
        let mut tool_count = 0u32;
        let tools_ptr = std::ptr::null_mut();
        let result = unsafe { (vtable.get_tools)(vtable.user_data, tools_ptr, &mut tool_count) };
        if result != 0 {
            anyhow::bail!("Failed to get tool count");
        }

        let mut tools_vec = Vec::with_capacity(tool_count as usize);
        let mut v: Vec<std::mem::MaybeUninit<ToolDefinition>> = (0..tool_count as usize)
            .map(|_| std::mem::MaybeUninit::uninit())
            .collect();
        let tools_ptr = v.as_mut_ptr() as *mut ToolDefinition;
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
                init_fn: PluginVTablePtr(vtable_ptr),
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

    /// Native plugins are disabled in default builds; rebuild explicitly with
    /// `--features native-plugins` only for reviewed, trusted libraries.
    #[cfg(not(feature = "native-plugins"))]
    pub async fn load_native_plugin(&self, lib_name: &str) -> Result<()> {
        let _ = lib_name;
        anyhow::bail!(
            "native plugins are disabled; rebuild with --features native-plugins to opt in"
        )
    }

    /// Load a WASM plugin (.wasm) with optional manifest/tools JSON override
    pub async fn load_wasm_plugin(
        &self,
        module_name: &str,
        manifest_json: Option<serde_json::Value>,
        tools_json: Option<serde_json::Value>,
    ) -> Result<()> {
        let module_path = self.wasm_module_dir.join(module_name);
        info!("Loading WASM plugin: {:?}", module_path);

        let module = Module::from_file(&self.wasm_engine, &module_path)?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(64 * 1024 * 1024)
            .table_elements(10_000)
            .build();
        let mut store = Store::new(&self.wasm_engine, limits);
        store.limiter(|limits| limits);
        store.set_fuel(1_000_000)?;

        // Create linker with host functions
        let mut linker = Linker::new(&self.wasm_engine);
        self.add_host_functions(&mut linker)?;

        // Instantiate asynchronously (required when async_support is enabled)
        let instance = linker.instantiate_async(&mut store, &module).await?;

        // Get allocate function to allocate memory in WASM for JSON data
        let allocate = instance.get_typed_func::<(i32,), (i32,)>(&mut store, "allocate")?;

        // Get set_manifest and set_tools functions (exported by WASM)
        let set_manifest = instance.get_typed_func::<(i32, i32), ()>(&mut store, "set_manifest")?;
        let set_tools = instance.get_typed_func::<(i32, i32), ()>(&mut store, "set_tools")?;

        // Write manifest JSON: allocate in WASM memory, copy data, register pointer
        let manifest_json = manifest_json.unwrap_or_else(|| serde_json::json!({}));
        let manifest_bytes = serde_json::to_vec(&manifest_json)?;
        let manifest_ptr = allocate
            .call_async(&mut store, (manifest_bytes.len() as i32,))
            .await?
            .0;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow::anyhow!("WASM plugin does not export memory"))?;
        memory.data_mut(&mut store)
            [manifest_ptr as usize..manifest_ptr as usize + manifest_bytes.len()]
            .copy_from_slice(&manifest_bytes);
        set_manifest
            .call_async(&mut store, (manifest_ptr, manifest_bytes.len() as i32))
            .await?;

        // Write tools JSON: allocate in WASM memory, copy data, register pointer
        let tools_json = tools_json.unwrap_or_else(|| serde_json::json!([]));
        let tools_bytes = serde_json::to_vec(&tools_json)?;
        let tools_ptr = allocate
            .call_async(&mut store, (tools_bytes.len() as i32,))
            .await?
            .0;
        memory.data_mut(&mut store)[tools_ptr as usize..tools_ptr as usize + tools_bytes.len()]
            .copy_from_slice(&tools_bytes);
        set_tools
            .call_async(&mut store, (tools_ptr, tools_bytes.len() as i32))
            .await?;

        // Get exports (now using the set JSON data)
        let get_tools = instance.get_typed_func::<(), (i32, i32)>(&mut store, "get_tools")?;
        let (tools_ptr_out, tools_len) = get_tools.call_async(&mut store, ()).await?;

        // Read tools from WASM memory
        let tools_data = &memory.data(&store)
            [tools_ptr_out as usize..tools_ptr_out as usize + tools_len as usize];
        let _tools: Vec<ToolDefinition> = serde_json::from_slice(tools_data)?;

        // Get manifest
        let get_manifest = instance.get_typed_func::<(), (i32, i32)>(&mut store, "get_manifest")?;
        let (manifest_ptr_out, manifest_len) = get_manifest.call_async(&mut store, ()).await?;
        let manifest_data = &memory.data(&store)
            [manifest_ptr_out as usize..manifest_ptr_out as usize + manifest_len as usize];
        let manifest: PluginManifest = serde_json::from_slice(manifest_data)?;

        let instance_id = {
            let mut id = self.next_instance_id.lock().await;
            *id += 1;
            *id
        };

        let plugin = Plugin {
            manifest,
            plugin_type: PluginType::Wasm {
                module,
                instance_id,
            },
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

    fn add_host_functions(&self, linker: &mut Linker<StoreLimits>) -> Result<()> {
        let host = self.wasm_host.clone();

        // db_read
        linker.func_wrap(
            "host",
            "db_read",
            move |_caller: Caller<'_, StoreLimits>,
                  _ptr: i32,
                  _len: i32,
                  _out_ptr: i32,
                  _out_len: i32|
                  -> i32 {
                // Implementation would read from caller memory
                0
            },
        )?;

        // fact_write
        let _host_fw = host.fact_write.clone();
        linker.func_wrap(
            "host",
            "fact_write",
            move |_name_ptr: i32, _name_len: i32, _text_ptr: i32, _text_len: i32| -> i64 {
                // Would need to read from WASM memory
                0
            },
        )?;

        // fact_read
        let _host_fr = host.fact_read.clone();
        linker.func_wrap(
            "host",
            "fact_read",
            move |_name_ptr: i32, _name_len: i32, _out_ptr: i32, _out_len: i32| -> i32 { 0 },
        )?;

        // entity_create
        let _host_ec = host.entity_create.clone();
        linker.func_wrap(
            "host",
            "entity_create",
            move |_name_ptr: i32,
                  _name_len: i32,
                  _type_ptr: i32,
                  _type_len: i32,
                  _project_ptr: i32,
                  _project_len: i32,
                  _obs_ptr: i32,
                  _obs_len: i32|
                  -> i64 { 0 },
        )?;

        // task_create
        let _host_tc = host.task_create.clone();
        linker.func_wrap(
            "host",
            "task_create",
            move |_title_ptr: i32,
                  _title_len: i32,
                  _body_ptr: i32,
                  _body_len: i32,
                  _status_ptr: i32,
                  _status_len: i32,
                  _project_ptr: i32,
                  _project_len: i32,
                  _priority_ptr: i32,
                  _priority_len: i32|
                  -> i64 { 0 },
        )?;

        // model_predict
        let _host_mp = host.model_predict.clone();
        linker.func_wrap(
            "host",
            "model_predict",
            move |_xs_ptr: i32,
                  _xs_len: i32,
                  _ys_ptr: i32,
                  _ys_len: i32,
                  _out_ptr: i32,
                  _out_len: i32|
                  -> i32 { 0 },
        )?;

        // vector_search
        let _host_vs = host.vector_search.clone();
        linker.func_wrap(
            "host",
            "vector_search",
            move |_query_ptr: i32, _query_len: i32, _k: i32, _out_ptr: i32, _out_len: i32| -> i32 {
                0
            },
        )?;

        Ok(())
    }

    /// Call a plugin tool
    pub async fn call_tool(
        &self,
        plugin_name: &str,
        tool_name: &str,
        args: Value,
    ) -> Result<Value> {
        let mut plugin = self
            .plugins
            .get_mut(plugin_name)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_name))?;

        if !plugin.state.enabled {
            anyhow::bail!("Plugin disabled: {}", plugin_name);
        }

        // Find tool definition
        let _tool_def = plugin.manifest.tools.iter().find(|t| t.name == tool_name);
        let hash_alias = plugin.manifest.name == "hash-tools"
            && matches!(tool_name, "md5" | "sha1" | "sha256" | "sha512");
        if _tool_def.is_none() && !hash_alias {
            anyhow::bail!("Tool not found: {}", tool_name);
        }

        // Validate args against schema (simplified)
        // In production, use jsonschema crate

        let result = match &plugin.plugin_type {
            #[cfg(feature = "native-plugins")]
            PluginType::Native { .. } => self.call_native_tool(&plugin, tool_name, args).await?,
            PluginType::Wasm {
                module: _,
                instance_id,
            } => {
                self.call_wasm_tool(&plugin, tool_name, args, *instance_id)
                    .await?
            }
        };

        plugin.state.call_count += 1;
        Ok(result)
    }

    #[cfg(feature = "native-plugins")]
    async fn call_native_tool(
        &self,
        _plugin: &Plugin,
        _tool_name: &str,
        _args: Value,
    ) -> Result<Value> {
        // Native plugin calling would use the vtable
        // Simplified for now
        Ok(Value::Null)
    }

    async fn call_wasm_tool(
        &self,
        plugin: &Plugin,
        tool_name: &str,
        args: Value,
        _instance_id: u64,
    ) -> Result<Value> {
        // Builtins have a deliberately small, deterministic host implementation.
        // Their WASM module carries the validated manifest and tool schema; keeping
        // cryptographic primitives in Rust avoids a fake no-op WASM call export.
        if plugin.manifest.name == "hash-tools" {
            let algorithm = if tool_name == "hash" {
                args.get("algorithm")
                    .and_then(Value::as_str)
                    .unwrap_or("sha256")
            } else {
                tool_name
            };
            let input = args
                .get("input")
                .or_else(|| args.get("text"))
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("hash tool requires string 'input' or 'text'"))?;
            let bytes = input.as_bytes();
            let digest = match algorithm {
                "md5" => format!("{:x}", Md5::digest(bytes)),
                "sha1" => format!("{:x}", Sha1::digest(bytes)),
                "sha256" => format!("{:x}", Sha256::digest(bytes)),
                "sha512" => format!("{:x}", Sha512::digest(bytes)),
                other => anyhow::bail!("Unsupported hash algorithm: {}", other),
            };
            return Ok(serde_json::json!({"hash": digest, "algorithm": algorithm}));
        }
        anyhow::bail!(
            "WASM tool execution is not implemented for plugin '{}'",
            plugin.manifest.name
        )
    }

    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginManifest> {
        self.plugins.iter().map(|p| p.manifest.clone()).collect()
    }

    /// Enable/disable plugin
    pub fn set_plugin_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        let mut plugin = self
            .plugins
            .get_mut(name)
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

        // Native dynamic libraries are never loaded by default.
        #[cfg(feature = "native-plugins")]
        if self.native_lib_dir.exists() {
            for entry in std::fs::read_dir(&self.native_lib_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path
                    .extension()
                    .is_some_and(|e| e == "so" || e == "dll" || e == "dylib")
                {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        if self.load_native_plugin(name).await.is_ok() {
                            loaded.push(name.to_string());
                        }
                    }
                }
            }
        }

        // Load WASM plugins (external .wasm files without registry JSON)
        if self.wasm_module_dir.exists() {
            for entry in std::fs::read_dir(&self.wasm_module_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "wasm") {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if self.load_wasm_plugin(name, None, None).await.is_ok() {
                            loaded.push(name.to_string());
                        }
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// Load a builtin plugin with JSON from registry
    pub async fn load_builtin_plugin(
        &self,
        name: &str,
        registry: &crate::plugin_marketplace::PluginRegistry,
    ) -> Result<()> {
        let manifest_json = Some(registry.get_builtin_manifest(name));
        let tools_json = Some(registry.get_builtin_tools(name));
        // Pass name with .wasm extension for file loading
        let filename = if name.ends_with(".wasm") {
            name.to_string()
        } else {
            format!("{}.wasm", name)
        };
        self.load_wasm_plugin(&filename, manifest_json, tools_json)
            .await
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
        let root =
            std::env::temp_dir().join(format!("koala-plugin-manager-{}", uuid::Uuid::new_v4()));
        let host = WasmHost::new();
        let manager = PluginManager::new(host, root.join("native"), root.join("wasm")).unwrap();
        assert!(manager.list_plugins().is_empty());
    }

    #[tokio::test]
    async fn wasm_memory_limit_rejects_oversized_modules() {
        let root = std::env::temp_dir().join(format!("koala-wasm-limits-{}", uuid::Uuid::new_v4()));
        let wasm_dir = root.join("wasm");
        std::fs::create_dir_all(&wasm_dir).unwrap();
        let bytes = wat::parse_str("(module (memory 1025))").unwrap();
        std::fs::write(wasm_dir.join("oversized.wasm"), bytes).unwrap();
        let manager = PluginManager::new(WasmHost::new(), root.join("native"), &wasm_dir).unwrap();
        assert!(manager
            .load_wasm_plugin("oversized.wasm", None, None)
            .await
            .is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn installed_hash_builtin_restores_manifest_and_computes_sha256() {
        let root = std::env::temp_dir().join(format!("koala-hash-plugin-{}", uuid::Uuid::new_v4()));
        let registry = crate::plugin_marketplace::PluginRegistry::new(&root)
            .await
            .unwrap();
        registry.install("hash-tools", None).await.unwrap();

        let manager = PluginManager::new(
            WasmHost::new(),
            root.join("plugins/native"),
            root.join("plugins/wasm"),
        )
        .unwrap();
        manager.discover_plugins().await.unwrap();
        manager
            .load_builtin_plugin("hash-tools", &registry)
            .await
            .unwrap();

        let manifest = manager
            .list_plugins()
            .into_iter()
            .find(|manifest| manifest.name == "hash-tools")
            .expect("builtin manifest is restored");
        assert!(manifest.tools.iter().any(|tool| tool.name == "hash"));

        let result = manager
            .call_tool(
                "hash-tools",
                "sha256",
                serde_json::json!({"input": "hello"}),
            )
            .await
            .unwrap();
        assert_eq!(
            result["hash"],
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
