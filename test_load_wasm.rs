#![allow(clippy::all)]

use peabrain_rust::{PluginManager, WasmHost};
use std::path::PathBuf;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let state_dir = PathBuf::from("/home/deeone/.peabrain");
    std::fs::create_dir_all(state_dir.join("plugins/wasm"))?;
    std::fs::create_dir_all(state_dir.join("plugins/native"))?;

    let wasm_host = WasmHost::new();
    let plugin_manager = PluginManager::new(
        wasm_host,
        state_dir.join("plugins/native"),
        state_dir.join("plugins/wasm"),
    )?;

    // Test loading hash-tools.wasm
    println!("Loading hash-tools.wasm...");
    match plugin_manager
        .load_wasm_plugin("hash-tools.wasm", None, None)
        .await
    {
        Ok(_) => println!("SUCCESS: Plugin loaded!"),
        Err(e) => println!("ERROR: {}", e),
    }

    // List loaded plugins
    let plugins = plugin_manager.list_plugins();
    println!("Loaded plugins: {:?}", plugins);

    Ok(())
}
