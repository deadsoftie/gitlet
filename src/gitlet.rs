use std::path::Path;

use anyhow::{anyhow, Context};
use chrono::Utc;

use crate::config::{self, GitletConfig, GitletEntry};
use crate::exclude;

pub fn init(git_root: &Path, name: &str) -> anyhow::Result<()> {
    let gitlet_dir = git_root.join(".gitlet").join(name);

    if gitlet_dir.exists() {
        return Err(anyhow!(
            "gitlet '{}' already exists. Run 'gitlet list' to see all gitlets.",
            name
        ));
    }

    // Create the bare git repo for this gitlet
    std::fs::create_dir_all(&gitlet_dir)
        .with_context(|| format!("failed to create {}", gitlet_dir.display()))?;
    git2::Repository::init_bare(&gitlet_dir)
        .with_context(|| format!("failed to init bare repo at {}", gitlet_dir.display()))?;

    // Create or update .gitlet/config.toml
    let gitlet_root = git_root.join(".gitlet");
    std::fs::create_dir_all(&gitlet_root)
        .with_context(|| format!("failed to create {}", gitlet_root.display()))?;

    let mut cfg = if gitlet_root.join("config.toml").exists() {
        config::load(git_root)?
    } else {
        GitletConfig::default()
    };

    cfg.gitlets.insert(
        name.to_string(),
        GitletEntry {
            created: Utc::now().to_rfc3339(),
        },
    );

    if cfg.active.is_empty() {
        cfg.active = name.to_string();
    }

    config::save(git_root, &cfg)?;

    // Add .gitlet/ to .git/info/exclude (idempotent)
    exclude::add_exclusion(git_root, ".gitlet/")?;

    println!("Initialized gitlet '{}'", name);
    Ok(())
}
