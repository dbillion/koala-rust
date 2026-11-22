use super::super::{missing_arg, KoalaServer};
use anyhow::Result;
use serde_json::{json, Value};

impl KoalaServer {
    pub(in crate::mcp) async fn handle_model_init(&self, args: Value) -> Result<Value> {
        let mut brain = self.brain.lock().await;
        let hidden = args.get("hidden").and_then(|v| v.as_u64()).unwrap_or(16) as usize;
        let lr = args.get("lr").and_then(|v| v.as_f64()).unwrap_or(0.01) as f32;
        let activation = args
            .get("activation")
            .and_then(|v| v.as_str())
            .unwrap_or("leaky");
        brain.init_model(hidden, lr, activation).await?;
        Ok(json!({"ok": true, "hidden": hidden, "activation": activation, "step": 0}))
    }

    pub(in crate::mcp) async fn handle_model_train(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let model = brain.model.lock().await;
        let mut model_guard = model;
        let model_ref = model_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("no model loaded (model_init or model_load first)"))?;

        let xs: Vec<f32> = args["xs"]
            .as_array()
            .ok_or_else(|| missing_arg("xs"))?
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect::<Vec<f32>>();
        let ys: Vec<f32> = args["ys"]
            .as_array()
            .ok_or_else(|| missing_arg("ys"))?
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect::<Vec<f32>>();
        let targets: Vec<f32> = args["targets"]
            .as_array()
            .ok_or_else(|| missing_arg("targets"))?
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect::<Vec<f32>>();
        let lr = args.get("lr").and_then(|v| v.as_f64()).map(|x| x as f32);
        let steps = args.get("steps").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

        if let Some(lr) = lr {
            model_ref.lr = lr;
        }

        let loss = model_ref.train_batch(&xs, &ys, &targets, steps)?;
        Ok(json!({"ok": true, "step": model_ref.step, "train_loss": loss}))
    }

    pub(in crate::mcp) async fn handle_model_predict(&self, args: Value) -> Result<Value> {
        let brain = self.brain.lock().await;
        let model = brain.model.lock().await;
        let model_ref = model
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no model loaded (model_init or model_load first)"))?;

        let xs: Vec<f32> = args["xs"]
            .as_array()
            .ok_or_else(|| missing_arg("xs"))?
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect();
        let ys: Vec<f32> = args["ys"]
            .as_array()
            .ok_or_else(|| missing_arg("ys"))?
            .iter()
            .filter_map(|v| v.as_f64().map(|x| x as f32))
            .collect();
        let values = model_ref.predict(&xs, &ys);
        Ok(json!({"ok": true, "values": values}))
    }
}
