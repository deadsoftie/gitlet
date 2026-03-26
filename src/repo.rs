use std::path::PathBuf;

use anyhow::anyhow;

pub fn find_git_root() -> anyhow::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;

    loop {
        if dir.join(".git").is_dir() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return Err(anyhow!("Not inside a git repository")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn set_cwd(path: &std::path::Path) {
        std::env::set_current_dir(path).expect("failed to set cwd");
    }

    #[test]
    fn finds_root_from_nested_subdir() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create a bare .git dir to simulate a git repo
        fs::create_dir(root.join(".git")).unwrap();

        // Create a nested subdirectory
        let nested = root.join("a/b/c");
        fs::create_dir_all(&nested).unwrap();

        set_cwd(&nested);
        let found = find_git_root().unwrap();
        assert_eq!(found.canonicalize().unwrap(), root.canonicalize().unwrap());
    }

    #[test]
    fn errors_outside_git_repo() {
        let tmp = TempDir::new().unwrap();
        // No .git directory — bare temp dir
        set_cwd(tmp.path());
        let err = find_git_root().unwrap_err();
        assert_eq!(err.to_string(), "Not inside a git repository");
    }
}
