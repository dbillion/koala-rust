#[cfg(feature = "net")]
use anyhow::Context;
use anyhow::Result;
use std::path::Path;

#[cfg(feature = "net")]
pub fn fetch_model_to(model_id: &str, destination: &Path) -> Result<()> {
    if std::env::var_os("KOALA_OFFLINE").is_some() {
        anyhow::bail!("model fetch requires network access; omit --offline");
    }

    let api = hf_hub::api::sync::Api::new().context("initialize Hugging Face Hub client")?;
    let repo = api.model(model_id.to_string());
    std::fs::create_dir_all(destination)
        .with_context(|| format!("create model directory {}", destination.display()))?;

    for filename in ["config.json", "tokenizer.json", "model.safetensors"] {
        let cached = repo
            .get(filename)
            .with_context(|| format!("download {filename} from {model_id}"))?;
        let target = destination.join(filename);
        std::fs::copy(&cached, &target)
            .with_context(|| format!("copy {filename} into {}", destination.display()))?;
    }

    model2vec_rs::model::StaticModel::from_pretrained(destination, None, None, None)
        .context("downloaded files are not a valid local Model2Vec model")?;
    Ok(())
}

#[cfg(not(feature = "net"))]
pub fn fetch_model_to(_model_id: &str, _destination: &Path) -> Result<()> {
    anyhow::bail!("model fetch requires a binary built with the net feature")
}
