#![allow(clippy::all)]

use std::fs;

fn main() -> anyhow::Result<()> {
    let plugins = ["hash-tools", "text-transform", "json-tools"];
    for name in plugins {
        let wat_path = format!("plugins/wasm/{}.wat", name);
        let wasm_path = format!("plugins/wasm/{}.wasm", name);
        let wat_source = fs::read_to_string(&wat_path)?;
        let wasm_bytes = wat::parse_str(&wat_source)?;
        fs::write(&wasm_path, &wasm_bytes)?;
        println!(
            "Compiled {} -> {} ({} bytes)",
            wat_path,
            wasm_path,
            wasm_bytes.len()
        );
    }
    Ok(())
}
