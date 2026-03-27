use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tempfile::TempDir;

// ── helpers ─────────────────────────────────────────────────────────────────

fn setup() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    git2::Repository::init(tmp.path()).unwrap();
    let root = tmp.path().canonicalize().unwrap();
    (tmp, root)
}

fn make_file(root: &Path, name: &str, content: &str) -> String {
    let path = root.join(name);
    fs::write(&path, content).unwrap();
    path.to_string_lossy().into_owned()
}

fn index_has(root: &Path, gitlet_name: &str, rel: &str) -> bool {
    let repo = git2::Repository::open(root.join(".gitlet").join(gitlet_name)).unwrap();
    repo.index().unwrap().get_path(Path::new(rel), 0).is_some()
}

fn has_exclude(root: &Path, pattern: &str) -> bool {
    gitlet::exclude::has_exclusion(root, pattern).unwrap()
}

// ── Test 1: Happy path ───────────────────────────────────────────────────────

#[test]
fn happy_path_init_add_commit_status_log() {
    let (_tmp, root) = setup();

    gitlet::gitlet::init(&root, "default").unwrap();

    let file = make_file(&root, "notes.md", "hello world");
    gitlet::gitlet::add(&root, &[file], None).unwrap();

    // status shows new file before commit
    gitlet::gitlet::status(&root, Some("default")).unwrap();

    gitlet::gitlet::commit(&root, "initial commit", None).unwrap();

    // status is clean after commit
    gitlet::gitlet::status(&root, Some("default")).unwrap();

    // log shows commit history without error
    gitlet::gitlet::log(&root, Some("default")).unwrap();

    // Verify commit actually exists in the bare repo
    let repo = git2::Repository::open(root.join(".gitlet/default")).unwrap();
    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head_commit.message().unwrap(), "initial commit");
    assert_eq!(head_commit.parent_count(), 0);
}

// ── Test 2: Multiple gitlets ─────────────────────────────────────────────────

#[test]
fn multiple_gitlets_separate_tracking() {
    let (_tmp, root) = setup();

    gitlet::gitlet::init(&root, "personal").unwrap();
    gitlet::gitlet::init(&root, "secrets").unwrap();

    let notes = make_file(&root, "notes.md", "my notes");
    let env_file = make_file(&root, ".env", "SECRET=xyz");

    gitlet::gitlet::add(&root, &[notes], Some("personal")).unwrap();
    gitlet::gitlet::add(&root, &[env_file], Some("secrets")).unwrap();

    // Each file lives only in the intended gitlet
    assert!(index_has(&root, "personal", "notes.md"));
    assert!(!index_has(&root, "personal", ".env"));
    assert!(index_has(&root, "secrets", ".env"));
    assert!(!index_has(&root, "secrets", "notes.md"));

    // Commits are independent
    gitlet::gitlet::commit(&root, "add notes", Some("personal")).unwrap();
    gitlet::gitlet::commit(&root, "add secret", Some("secrets")).unwrap();

    let personal_repo = git2::Repository::open(root.join(".gitlet/personal")).unwrap();
    let secrets_repo = git2::Repository::open(root.join(".gitlet/secrets")).unwrap();
    assert_eq!(
        personal_repo.head().unwrap().peel_to_commit().unwrap().message().unwrap(),
        "add notes"
    );
    assert_eq!(
        secrets_repo.head().unwrap().peel_to_commit().unwrap().message().unwrap(),
        "add secret"
    );

    // list shows both
    gitlet::gitlet::list(&root).unwrap();
}

// ── Test 3: Exclude hygiene ──────────────────────────────────────────────────

#[test]
fn exclude_hygiene_add_then_remove() {
    let (_tmp, root) = setup();

    gitlet::gitlet::init(&root, "default").unwrap();
    let file = make_file(&root, "secret.txt", "password");

    gitlet::gitlet::add(&root, &[file.clone()], None).unwrap();

    // After add: file is in exclude
    assert!(has_exclude(&root, "secret.txt"));

    gitlet::gitlet::remove(&root, &file, None).unwrap();

    // After remove: file is gone from exclude
    assert!(!has_exclude(&root, "secret.txt"));
    assert!(!index_has(&root, "default", "secret.txt"));
}

// ── Test 4: Active switching ─────────────────────────────────────────────────

#[test]
fn active_switching_affects_default_target() {
    let (_tmp, root) = setup();

    gitlet::gitlet::init(&root, "alpha").unwrap();
    gitlet::gitlet::init(&root, "beta").unwrap();

    // Active is "alpha" (first created)
    let file_a = make_file(&root, "a.txt", "content a");
    gitlet::gitlet::add(&root, &[file_a], None).unwrap();
    assert!(index_has(&root, "alpha", "a.txt"));
    assert!(!index_has(&root, "beta", "a.txt"));

    // Switch active to "beta"
    gitlet::gitlet::switch(&root, "beta").unwrap();
    let cfg = gitlet::config::load(&root).unwrap();
    assert_eq!(cfg.active, "beta");

    // Now add without --to goes to beta
    let file_b = make_file(&root, "b.txt", "content b");
    gitlet::gitlet::add(&root, &[file_b], None).unwrap();
    assert!(index_has(&root, "beta", "b.txt"));
    assert!(!index_has(&root, "alpha", "b.txt"));

    // list reflects the new active
    gitlet::gitlet::list(&root).unwrap();
}

// ── Test 5: Outer git isolation ──────────────────────────────────────────────

#[test]
fn outer_git_isolation_file_excluded_from_outer_git() {
    let (_tmp, root) = setup();

    gitlet::gitlet::init(&root, "default").unwrap();
    let file = make_file(&root, "private.txt", "private data");

    gitlet::gitlet::add(&root, &[file], None).unwrap();

    // File is in exclude, so outer git treats it as ignored
    assert!(has_exclude(&root, "private.txt"));

    // Verify via git2: private.txt must NOT appear as untracked in outer git
    let outer = git2::Repository::open(&root).unwrap();
    let statuses = outer.statuses(None).unwrap();
    let is_untracked = statuses.iter().any(|e| {
        e.status().is_wt_new() && e.path().unwrap_or("") == "private.txt"
    });
    assert!(!is_untracked, "private.txt should be excluded from outer git, not untracked");
}

// ── Test 6: Not a git repo ───────────────────────────────────────────────────

// Serialise any test that mutates the process working directory.
static CWD_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn not_a_git_repo_find_git_root_errors() {
    // Create a plain temp dir with no git repo inside it
    let tmp = TempDir::new().unwrap();
    let tmp_path = tmp.path().canonicalize().unwrap();

    let _guard = CWD_MUTEX.lock().unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(&tmp_path).unwrap();

    let result = gitlet::repo::find_git_root();

    // Always restore before asserting so we don't leave the process in a bad state
    std::env::set_current_dir(original).unwrap();

    assert!(
        result.is_err(),
        "find_git_root() should error outside a git repo"
    );
    let msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        msg.contains("git") || msg.contains("repository"),
        "error message should mention 'git' or 'repository', got: {msg}"
    );
}
