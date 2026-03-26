use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct GitletConfig {
    #[serde(default)]
    pub active: String,
    #[serde(default)]
    pub gitlets: HashMap<String, GitletEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct GitletEntry {
    pub created: String,
}

fn config_path(root: &Path) -> std::path::PathBuf {
    root.join(".gitlet").join("config.toml")
}

pub fn load(root: &Path) -> anyhow::Result<GitletConfig> {
    let path = config_path(root);
    if !path.exists() {
        return Err(anyhow!(
            "No gitlets found. Run 'gitlet init' to create one."
        ));
    }
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn save(root: &Path, config: &GitletConfig) -> anyhow::Result<()> {
    let path = config_path(root);
    let contents = toml::to_string_pretty(config).context("failed to serialize config")?;

    // Atomic write: write to a temp file then rename
    let tmp_path = path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, &contents)
        .with_context(|| format!("failed to write {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &path)
        .with_context(|| format!("failed to rename temp config to {}", path.display()))?;

    Ok(())
}

pub fn get_active(root: &Path) -> anyhow::Result<String> {
    let config = load(root)?;
    Ok(config.active.clone())
}

pub fn set_active(root: &Path, name: &str) -> anyhow::Result<()> {
    let mut config = load(root)?;
    config.active = name.to_string();
    save(root, &config)
}
