use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct GitnookConfig {
    #[serde(default)]
    pub active: String,
    #[serde(default)]
    pub gitnooks: HashMap<String, GitnookEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitnookEntry {
    pub created: String,
}

fn config_path(root: &Path) -> PathBuf {
    root.join(".gitnook").join("config.toml")
}

pub fn load(root: &Path) -> anyhow::Result<GitnookConfig> {
    let path = config_path(root);
    if !path.exists() {
        return Err(anyhow!(
            "No gitnooks found. Run 'gitnook init' first."
        ));
    }
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn save(root: &Path, config: &GitnookConfig) -> anyhow::Result<()> {
    let path = config_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let contents = toml::to_string_pretty(config).context("failed to serialize config")?;

    // Atomic write: write to a temp file then rename
    let tmp_path = path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, &contents)
        .with_context(|| format!("failed to write {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &path)
        .with_context(|| format!("failed to rename temp config to {}", path.display()))?;

    Ok(())
}

pub fn set_active(root: &Path, name: &str) -> anyhow::Result<()> {
    let mut config = load(root)?;
    config.active = name.to_string();
    save(root, &config)
}

#[cfg(test)]
#[path = "tests/config_tests.rs"]
mod tests;
