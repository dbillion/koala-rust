#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > 1024 * 1024 {
        return;
    }
    if let Ok(line) = std::str::from_utf8(data) {
        if let Some(request) = koala_rust::mcp::parse_jsonrpc_request(line) {
            let _ = request.get("method").and_then(serde_json::Value::as_str);
            let _ = serde_json::to_vec(&request);
        }
    }
});
