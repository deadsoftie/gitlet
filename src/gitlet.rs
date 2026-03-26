use std::path::{Path, PathBuf};

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

pub fn add(git_root: &Path, files: &[String], to: Option<&str>) -> anyhow::Result<()> {
    let cfg = config::load(git_root)?;
    let target = to.unwrap_or(&cfg.active).to_string();

    let gitlet_dir = git_root.join(".gitlet").join(&target);
    if !gitlet_dir.exists() {
        return Err(anyhow!("gitlet '{}' does not exist.", target));
    }

    let repo = git2::Repository::open(&gitlet_dir)
        .with_context(|| format!("failed to open gitlet repo at {}", gitlet_dir.display()))?;

    for file in files {
        let abs = resolve_file(git_root, file)?;
        let rel = abs
            .strip_prefix(git_root)
            .with_context(|| format!("'{}' is outside the git repo", file))?
            .to_path_buf();

        // Warn if tracked by the outer git
        if is_tracked_by_outer_git(git_root, &rel)? {
            eprintln!(
                "Warning: {} is tracked by git. To fully remove it run: git rm --cached {}",
                rel.display(),
                rel.display()
            );
        }

        // Error if already in any gitlet
        if let Some(owner) = find_owning_gitlet(git_root, &cfg, &rel)? {
            return Err(anyhow!(
                "{} is already tracked by gitlet '{}'",
                rel.display(),
                owner
            ));
        }

        // Stage in the target gitlet index.
        // Bare repos have no workdir, so we create a blob from the real file
        // and add it to the index manually.
        let abs = git_root.join(&rel);
        let blob_id = repo
            .blob_path(&abs)
            .with_context(|| format!("failed to create blob for {}", abs.display()))?;

        let mut index = repo.index().context("failed to get gitlet index")?;
        let entry = git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: blob_id,
            flags: 0,
            flags_extended: 0,
            path: rel.to_string_lossy().into_owned().into_bytes(),
        };
        index.add(&entry).with_context(|| {
            format!("failed to stage {} in gitlet '{}'", rel.display(), target)
        })?;
        index.write().context("failed to write gitlet index")?;

        // Add to .git/info/exclude
        exclude::add_exclusion(git_root, &rel.to_string_lossy())?;

        println!("Added {} to gitlet '{}'", rel.display(), target);
    }

    Ok(())
}

/// Resolve a file argument to an absolute path, erroring if it does not exist.
fn resolve_file(_git_root: &Path, file: &str) -> anyhow::Result<PathBuf> {
    let p = PathBuf::from(file);
    let abs = if p.is_absolute() {
        p
    } else {
        std::env::current_dir()?.join(p)
    };
    // Canonicalize resolves symlinks and `..` components
    abs.canonicalize()
        .with_context(|| format!("'{}' does not exist", file))
}

/// Check whether a relative path is currently staged in the outer git index.
fn is_tracked_by_outer_git(git_root: &Path, rel: &Path) -> anyhow::Result<bool> {
    let outer = git2::Repository::discover(git_root)
        .context("failed to open outer git repo")?;
    let index = outer.index().context("failed to read outer git index")?;
    Ok(index.get_path(rel, 0).is_some())
}

/// Return the name of the gitlet that already tracks `rel`, if any.
fn find_owning_gitlet(
    git_root: &Path,
    cfg: &GitletConfig,
    rel: &Path,
) -> anyhow::Result<Option<String>> {
    for name in cfg.gitlets.keys() {
        let gitlet_dir = git_root.join(".gitlet").join(name);
        if !gitlet_dir.exists() {
            continue;
        }
        if let Ok(repo) = git2::Repository::open(&gitlet_dir) {
            if let Ok(index) = repo.index() {
                if index.get_path(rel, 0).is_some() {
                    return Ok(Some(name.clone()));
                }
            }
        }
    }
    Ok(None)
}
