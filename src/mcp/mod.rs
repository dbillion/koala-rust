use std::sync::Arc;
use tokio::sync::Mutex;

use crate::Koala;

mod dispatch;
mod handlers;
mod server;
mod tools;

pub use server::{parse_jsonrpc_request, run_stdio};

fn missing_arg(key: &str) -> anyhow::Error {
    anyhow::anyhow!("missing or invalid argument: {}", key)
}

pub struct KoalaServer {
    brain: Arc<Mutex<Koala>>,
}

impl KoalaServer {
    pub fn new(brain: Koala) -> Self {
        Self {
            brain: Arc::new(Mutex::new(brain)),
        }
    }
}
