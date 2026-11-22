use super::KoalaServer;
use crate::Koala;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

impl KoalaServer {
    async fn handle_request(&self, request: Value) -> Value {
        let id = request.get("id").cloned().unwrap_or(json!(null));
        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(json!({}));

        let result = match method {
            "initialize" => self.handle_initialize(params).await,
            "tools/list" => self.handle_list_tools().await,
            "tools/call" => self.handle_tool_call(params).await,
            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        };

        match result {
            Ok(result) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            }),
            Err(e) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": e.to_string()
                }
            }),
        }
    }

    async fn handle_initialize(&self, _params: Value) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "koala-rust", "version": env!("CARGO_PKG_VERSION") }
        }))
    }
}

pub async fn run_stdio(brain: Koala) -> Result<()> {
    let server = Arc::new(KoalaServer {
        brain: Arc::new(Mutex::new(brain)),
    });
    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let request: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let response = server.handle_request(request).await;
        let response_str = serde_json::to_string(&response)?;
        stdout.write_all(response_str.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}
