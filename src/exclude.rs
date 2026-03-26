use std::path::Path;

use anyhow::Context;

fn exclude_path(git_root: &Path) -> std::path::PathBuf {
    git_root.join(".git").join("info").join("exclude")
}

pub fn has_exclusion(git_root: &Path, pattern: &str) -> anyhow::Result<bool> {
    let path = exclude_path(git_root);
    if !path.exists() {
        return Ok(false);
    }
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(contents.lines().any(|line| line == pattern))
}

pub fn add_exclusion(git_root: &Path, pattern: &str) -> anyhow::Result<()> {
    if has_exclusion(git_root, pattern)? {
        return Ok(());
    }

    let path = exclude_path(git_root);

    // Create .git/info/ directory and file if they don't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut contents = if path.exists() {
        std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?
    } else {
        "# gitlet managed entries\n".to_string()
    };

    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(pattern);
    contents.push('\n');

    std::fs::write(&path, &contents)
        .with_context(|| format!("failed to write {}", path.display()))
}

pub fn remove_exclusion(git_root: &Path, pattern: &str) -> anyhow::Result<()> {
    let path = exclude_path(git_root);
    if !path.exists() {
        return Ok(());
    }
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let filtered: Vec<&str> = contents.lines().filter(|l| *l != pattern).collect();
    let new_contents = filtered.join("\n") + "\n";
    std::fs::write(&path, new_contents)
        .with_context(|| format!("failed to write {}", path.display()))
}
