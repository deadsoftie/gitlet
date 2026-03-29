use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use chrono::{FixedOffset, TimeZone, Utc};

use crate::config::{self, GitnookConfig, GitnookEntry};
use crate::exclude;

pub fn init(git_root: &Path, name: &str) -> anyhow::Result<()> {
    let gitnook_dir = git_root.join(".gitnook").join(name);

    if gitnook_dir.exists() {
        return Err(anyhow!(
            "gitnook '{}' already exists. Run 'gitnook list' to see all gitnooks.",
            name
        ));
    }

    // Create the bare git repo for this gitnook (.gitnook/ is created as a side-effect)
    std::fs::create_dir_all(&gitnook_dir)
        .with_context(|| format!("failed to create {}", gitnook_dir.display()))?;
    git2::Repository::init_bare(&gitnook_dir)
        .with_context(|| format!("failed to init bare repo at {}", gitnook_dir.display()))?;

    // Create or update .gitnook/config.toml
    let gitnook_root = git_root.join(".gitnook");

    let mut cfg = if gitnook_root.join("config.toml").exists() {
        config::load(git_root)?
    } else {
        GitnookConfig::default()
    };

    cfg.gitnooks.insert(
        name.to_string(),
        GitnookEntry {
            created: Utc::now().to_rfc3339(),
        },
    );

    if cfg.active.is_empty() {
        cfg.active = name.to_string();
    }

    config::save(git_root, &cfg)?;

    // Add .gitnook/ to .git/info/exclude (idempotent)
    exclude::add_exclusion(git_root, ".gitnook/")?;

    println!("Initialized gitnook '{}'", name);
    Ok(())
}

pub fn add(git_root: &Path, files: &[String], to: Option<&str>) -> anyhow::Result<()> {
    // Canonicalize git_root so strip_prefix works correctly on macOS where
    // current_dir() may return a symlinked path (e.g. /var → /private/var).
    let git_root = git_root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", git_root.display()))?;
    let git_root = git_root.as_path();

    let cfg = config::load(git_root)?;
    let target = to.unwrap_or(&cfg.active).to_string();

    let gitnook_dir = git_root.join(".gitnook").join(&target);
    if !gitnook_dir.exists() {
        return Err(anyhow!("gitnook '{}' does not exist.", target));
    }

    let repo = git2::Repository::open(&gitnook_dir)
        .with_context(|| format!("failed to open gitnook repo at {}", gitnook_dir.display()))?;

    for file in files {
        // resolve_file canonicalizes; with git_root also canonical, strip_prefix is safe.
        let abs = resolve_file(file)?;
        let rel = abs
            .strip_prefix(git_root)
            .with_context(|| format!("'{}' is outside the git repo", file))?
            .to_path_buf();

        // Warn if tracked by the outer git
        if is_tracked_by_outer_git(git_root, &rel)? {
            eprintln!(
                "Warning: {} is currently tracked by git. Run: git rm --cached {}",
                rel.display(),
                rel.display()
            );
        }

        // Error only if the file is owned by a *different* gitnook.
        // Re-adding to the same gitnook is how the user stages modifications.
        if let Some(owner) = find_owning_gitnook(git_root, &cfg, &rel)? {
            if owner != target {
                return Err(anyhow!(
                    "{} is already tracked by gitnook '{}'",
                    rel.display(),
                    owner
                ));
            }
        }

        // Stage in the target gitnook index.
        // Bare repos have no workdir, so we create a blob from the real file
        // and add it to the index manually. Use the canonical abs path directly.
        let blob_id = repo
            .blob_path(&abs)
            .with_context(|| format!("failed to create blob for {}", abs.display()))?;

        let mut index = repo.index().context("failed to get gitnook index")?;
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
            format!("failed to stage {} in gitnook '{}'", rel.display(), target)
        })?;
        index.write().context("failed to write gitnook index")?;

        // Add to .git/info/exclude
        exclude::add_exclusion(git_root, &rel.to_string_lossy())?;

        println!("Added {} to gitnook '{}'", rel.display(), target);
    }

    Ok(())
}

pub fn remove(git_root: &Path, file: &str, to: Option<&str>) -> anyhow::Result<()> {
    // Canonicalize for consistency with `add` and correct strip_prefix on macOS.
    let git_root = git_root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", git_root.display()))?;
    let git_root = git_root.as_path();

    let cfg = config::load(git_root)?;
    let target = to.unwrap_or(&cfg.active).to_string();

    let gitnook_dir = git_root.join(".gitnook").join(&target);
    if !gitnook_dir.exists() {
        return Err(anyhow!("gitnook '{}' does not exist.", target));
    }

    // Resolve path — file may have been deleted from disk, so don't require it to exist
    let rel = rel_path(git_root, file)?;

    // Verify the file is tracked by this gitnook
    let repo = git2::Repository::open(&gitnook_dir)
        .with_context(|| format!("failed to open gitnook repo at {}", gitnook_dir.display()))?;
    let mut index = repo.index().context("failed to get gitnook index")?;

    if index.get_path(&rel, 0).is_none() {
        return Err(anyhow!(
            "'{}' is not tracked by gitnook '{}'",
            rel.display(),
            target
        ));
    }

    index
        .remove_path(&rel)
        .with_context(|| format!("failed to remove {} from gitnook index", rel.display()))?;
    index.write().context("failed to write gitnook index")?;

    exclude::remove_exclusion(git_root, &rel.to_string_lossy())?;

    println!(
        "Removed {} from gitnook '{}'. The file is now visible to git.",
        rel.display(),
        target
    );
    Ok(())
}

pub fn commit(git_root: &Path, message: &str, to: Option<&str>) -> anyhow::Result<()> {
    let git_root = git_root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", git_root.display()))?;
    let git_root = git_root.as_path();

    let cfg = config::load(git_root)?;
    let target = to.unwrap_or(&cfg.active).to_string();

    let gitnook_dir = git_root.join(".gitnook").join(&target);
    if !gitnook_dir.exists() {
        return Err(anyhow!("gitnook '{}' does not exist.", target));
    }

    let repo = git2::Repository::open(&gitnook_dir)
        .with_context(|| format!("failed to open gitnook repo at {}", gitnook_dir.display()))?;

    // Build the tree from the current index
    let mut index = repo.index().context("failed to get gitnook index")?;
    let tree_id = index.write_tree().context("failed to write index tree")?;
    let tree = repo.find_tree(tree_id).context("failed to find tree")?;

    // Read author/committer from the outer git config, with fallbacks
    let (author_name, author_email) = read_git_identity(git_root);
    let sig = git2::Signature::now(&author_name, &author_email)
        .context("failed to create git signature")?;

    // Create root commit or chained commit depending on whether HEAD resolves
    let oid = match repo.head() {
        Ok(head) => {
            let parent = head.peel_to_commit().context("failed to peel HEAD to commit")?;
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
                .context("failed to create commit")?
        }
        Err(_) => {
            // HEAD doesn't exist yet — this is the first commit
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .context("failed to create root commit")?
        }
    };

    let short_sha = &oid.to_string()[..7];
    println!("[{}] {} {}", target, short_sha, message);
    Ok(())
}

pub fn status(git_root: &Path, name: Option<&str>) -> anyhow::Result<()> {
    let git_root = git_root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", git_root.display()))?;
    let git_root = git_root.as_path();

    let cfg = match config::load(git_root) {
        Ok(c) if !c.gitnooks.is_empty() => c,
        _ => {
            println!("No gitnooks found. Run 'gitnook init' to create one.");
            return Ok(());
        }
    };

    // Collect the names to display, sorted for deterministic output
    let names: Vec<&str> = match name {
        Some(n) => {
            if !cfg.gitnooks.contains_key(n) {
                return Err(anyhow!("gitnook '{}' does not exist.", n));
            }
            vec![n]
        }
        None => {
            let mut v: Vec<&str> = cfg.gitnooks.keys().map(String::as_str).collect();
            v.sort();
            v
        }
    };

    // Width of the widest name, for column alignment
    let max_len = names.iter().map(|n| n.len()).max().unwrap_or(0);

    for name in &names {
        let label = format!("[{}]", name);
        let padding = " ".repeat(max_len - name.len() + 2);
        let summary = gitnook_status_summary(git_root, name)?;
        println!("{}{}{}", label, padding, summary);
    }

    Ok(())
}

/// Compute a one-line status summary for a single gitnook.
fn gitnook_status_summary(git_root: &Path, name: &str) -> anyhow::Result<String> {
    let gitnook_dir = git_root.join(".gitnook").join(name);
    let repo = git2::Repository::open(&gitnook_dir)
        .with_context(|| format!("failed to open gitnook repo '{}'", name))?;
    let index = repo.index().context("failed to read gitnook index")?;

    // Resolve HEAD tree once; None means no commits yet
    let head_tree = match repo.head() {
        Ok(head) => Some(head.peel_to_tree().context("failed to peel HEAD to tree")?),
        Err(_) => None,
    };

    let mut new_files: Vec<String> = Vec::new();
    let mut modified_files: Vec<String> = Vec::new();

    for i in 0..index.len() {
        let entry = match index.get(i) {
            Some(e) => e,
            None => continue,
        };
        let path_str = String::from_utf8_lossy(&entry.path).into_owned();
        let path = std::path::Path::new(&path_str);

        match &head_tree {
            // No commits yet — every indexed file is "new"
            None => new_files.push(path_str),
            Some(tree) => match tree.get_path(path) {
                // Not in the last commit → new (staged but not committed)
                Err(_) => new_files.push(path_str),
                // In the last commit — compare committed blob with on-disk content
                Ok(tree_entry) => {
                    let abs = git_root.join(path);
                    let on_disk = std::fs::read(&abs).unwrap_or_default();
                    if let Ok(blob) = repo.find_blob(tree_entry.id()) {
                        if blob.content() != on_disk.as_slice() {
                            modified_files.push(path_str);
                        }
                    }
                }
            },
        }
    }

    if new_files.is_empty() && modified_files.is_empty() {
        return Ok("clean".to_string());
    }

    let mut parts: Vec<String> = Vec::new();

    if !new_files.is_empty() {
        let label = if new_files.len() == 1 {
            format!("1 new file: {}", new_files[0])
        } else {
            format!("{} new files: {}", new_files.len(), new_files.join(", "))
        };
        parts.push(label);
    }

    for f in &modified_files {
        parts.push(format!("modified: {}", f));
    }

    Ok(parts.join(", "))
}

pub fn log(git_root: &Path, name: Option<&str>) -> anyhow::Result<()> {
    let git_root = git_root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", git_root.display()))?;
    let git_root = git_root.as_path();

    let cfg = config::load(git_root)?;
    let target = name.unwrap_or(&cfg.active);

    if !cfg.gitnooks.contains_key(target) {
        return Err(anyhow!("gitnook '{}' does not exist.", target));
    }

    let gitnook_dir = git_root.join(".gitnook").join(target);
    let repo = git2::Repository::open(&gitnook_dir)
        .with_context(|| format!("failed to open gitnook repo '{}'", target))?;

    // If HEAD doesn't resolve there are no commits yet
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => {
            println!("No commits yet in gitnook '{}'", target);
            return Ok(());
        }
    };

    let mut revwalk = repo.revwalk().context("failed to create revwalk")?;
    revwalk
        .push(head.peel_to_commit()?.id())
        .context("failed to push HEAD onto revwalk")?;
    revwalk
        .set_sorting(git2::Sort::TIME)
        .context("failed to set revwalk sort")?;

    for oid in revwalk {
        let oid = oid.context("failed to read commit oid")?;
        let commit = repo
            .find_commit(oid)
            .with_context(|| format!("failed to find commit {}", oid))?;

        let short_sha = &oid.to_string()[..7];

        let author = commit.author();
        let author_name = author.name().unwrap_or("unknown");
        let author_email = author.email().unwrap_or("");

        let time = commit.time();
        // east_opt(0) is always Some — 0 is always a valid UTC offset
        let utc = FixedOffset::east_opt(0).expect("UTC offset 0 is always valid");
        let offset = FixedOffset::east_opt(time.offset_minutes() * 60).unwrap_or(utc);
        let dt = offset
            .timestamp_opt(time.seconds(), 0)
            .single()
            .context("invalid commit timestamp")?;
        let date_str = dt.format("%a %b %e %H:%M:%S %Y").to_string();

        let message = commit.message().unwrap_or("").trim_end();

        println!("commit {}", short_sha);
        println!("Author: {} <{}>", author_name, author_email);
        println!("Date:   {}", date_str);
        println!();
        for line in message.lines() {
            println!("    {}", line);
        }
        println!();
    }

    Ok(())
}

pub fn list(git_root: &Path) -> anyhow::Result<()> {
    let git_root = git_root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", git_root.display()))?;
    let git_root = git_root.as_path();

    let cfg = match config::load(git_root) {
        Ok(c) if !c.gitnooks.is_empty() => c,
        _ => {
            println!("No gitnooks initialized in this repo. Run 'gitnook init'.");
            return Ok(());
        }
    };

    // Sort names for deterministic output
    let mut names: Vec<&str> = cfg.gitnooks.keys().map(String::as_str).collect();
    names.sort();

    let max_len = names.iter().map(|n| n.len()).max().unwrap_or(0);

    for name in &names {
        let is_active = *name == cfg.active;
        let marker = if is_active { "*" } else { " " };
        let active_label = if is_active { "(active)" } else { "        " };

        // Count files tracked in this gitnook's index
        let file_count = {
            let gitnook_dir = git_root.join(".gitnook").join(name);
            let repo = git2::Repository::open(&gitnook_dir)
                .with_context(|| format!("failed to open gitnook repo '{}'", name))?;
            repo.index()
                .with_context(|| format!("failed to read index for gitnook '{}'", name))?
                .len()
        };

        let file_label = if file_count == 1 {
            "1 file tracked".to_string()
        } else {
            format!("{} files tracked", file_count)
        };

        // Pad name column to align active_label and file counts
        let name_padding = " ".repeat(max_len - name.len() + 2);

        println!(
            "{} {}{}{}   {}",
            marker, name, name_padding, active_label, file_label
        );
    }

    Ok(())
}

/// Read user.name and user.email from the outer git config, falling back to defaults.
fn read_git_identity(git_root: &Path) -> (String, String) {
    let name;
    let email;

    match git2::Repository::discover(git_root).and_then(|r| r.config()) {
        Ok(cfg) => {
            name = cfg
                .get_string("user.name")
                .unwrap_or_else(|_| "gitnook user".to_string());
            email = cfg
                .get_string("user.email")
                .unwrap_or_else(|_| "gitnook@local".to_string());
        }
        Err(_) => {
            name = "gitnook user".to_string();
            email = "gitnook@local".to_string();
        }
    }

    (name, email)
}

/// Build a repo-relative path from a raw file argument without requiring the file to exist.
fn rel_path(git_root: &Path, file: &str) -> anyhow::Result<PathBuf> {
    let p = PathBuf::from(file);
    let abs = if p.is_absolute() {
        p
    } else {
        std::env::current_dir()?.join(&p)
    };
    // Normalise without hitting the filesystem (file may be deleted)
    let abs = normalize_path(&abs);
    abs.strip_prefix(git_root)
        .with_context(|| format!("'{}' is outside the git repo", file))
        .map(|p| p.to_path_buf())
}

/// Lexically normalise a path (resolve `.` and `..`) without hitting the filesystem.
fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            c => out.push(c),
        }
    }
    out
}

/// Resolve a file argument to a canonical absolute path, erroring if it does not exist.
fn resolve_file(file: &str) -> anyhow::Result<PathBuf> {
    let p = PathBuf::from(file);
    let abs = if p.is_absolute() {
        p
    } else {
        std::env::current_dir()?.join(p)
    };
    // canonicalize resolves symlinks and `..` so strip_prefix against a
    // canonicalized git_root is always safe.
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

/// Return the name of the gitnook that already tracks `rel`, if any.
fn find_owning_gitnook(
    git_root: &Path,
    cfg: &GitnookConfig,
    rel: &Path,
) -> anyhow::Result<Option<String>> {
    for name in cfg.gitnooks.keys() {
        let gitnook_dir = git_root.join(".gitnook").join(name);
        if !gitnook_dir.exists() {
            continue;
        }
        let repo = git2::Repository::open(&gitnook_dir)
            .with_context(|| format!("failed to open gitnook repo '{}'", name))?;
        let index = repo
            .index()
            .with_context(|| format!("failed to read index for gitnook '{}'", name))?;
        if index.get_path(rel, 0).is_some() {
            return Ok(Some(name.clone()));
        }
    }
    Ok(None)
}

pub fn switch(git_root: &Path, name: &str) -> anyhow::Result<()> {
    let cfg = config::load(git_root)?;

    if !cfg.gitnooks.contains_key(name) {
        return Err(anyhow!(
            "gitnook '{}' does not exist. Run 'gitnook list' to see all gitnooks.",
            name
        ));
    }

    config::set_active(git_root, name)?;
    println!("Switched active gitnook to '{}'", name);
    Ok(())
}

#[cfg(test)]
#[path = "tests/gitnook_tests.rs"]
mod tests;

