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

fn index_has(root: &Path, gitnook_name: &str, rel: &str) -> bool {
    let repo = git2::Repository::open(root.join(".gitnook").join(gitnook_name)).unwrap();
    repo.index().unwrap().get_path(Path::new(rel), 0).is_some()
}

fn has_exclude(root: &Path, pattern: &str) -> bool {
    gitnook::exclude::has_exclusion(root, pattern).unwrap()
}

// ── Test 1: Happy path ───────────────────────────────────────────────────────

#[test]
fn happy_path_init_add_commit_status_log() {
    let (_tmp, root) = setup();

    gitnook::gitnook::init(&root, "default").unwrap();

    let file = make_file(&root, "notes.md", "hello world");
    gitnook::gitnook::add(&root, &[file], None).unwrap();

    // status shows new file before commit
    gitnook::gitnook::status(&root, Some("default")).unwrap();

    gitnook::gitnook::commit(&root, "initial commit", None).unwrap();

    // status is clean after commit
    gitnook::gitnook::status(&root, Some("default")).unwrap();

    // log shows commit history without error
    gitnook::gitnook::log(&root, Some("default")).unwrap();

    // Verify commit actually exists in the bare repo
    let repo = git2::Repository::open(root.join(".gitnook/default")).unwrap();
    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head_commit.message().unwrap(), "initial commit");
    assert_eq!(head_commit.parent_count(), 0);
}

// ── Test 2: Multiple gitnooks ─────────────────────────────────────────────────

#[test]
fn multiple_gitnooks_separate_tracking() {
    let (_tmp, root) = setup();

    gitnook::gitnook::init(&root, "personal").unwrap();
    gitnook::gitnook::init(&root, "secrets").unwrap();

    let notes = make_file(&root, "notes.md", "my notes");
    let env_file = make_file(&root, ".env", "SECRET=xyz");

    gitnook::gitnook::add(&root, &[notes], Some("personal")).unwrap();
    gitnook::gitnook::add(&root, &[env_file], Some("secrets")).unwrap();

    // Each file lives only in the intended gitnook
    assert!(index_has(&root, "personal", "notes.md"));
    assert!(!index_has(&root, "personal", ".env"));
    assert!(index_has(&root, "secrets", ".env"));
    assert!(!index_has(&root, "secrets", "notes.md"));

    // Commits are independent
    gitnook::gitnook::commit(&root, "add notes", Some("personal")).unwrap();
    gitnook::gitnook::commit(&root, "add secret", Some("secrets")).unwrap();

    let personal_repo = git2::Repository::open(root.join(".gitnook/personal")).unwrap();
    let secrets_repo = git2::Repository::open(root.join(".gitnook/secrets")).unwrap();
    assert_eq!(
        personal_repo.head().unwrap().peel_to_commit().unwrap().message().unwrap(),
        "add notes"
    );
    assert_eq!(
        secrets_repo.head().unwrap().peel_to_commit().unwrap().message().unwrap(),
        "add secret"
    );

    // list shows both
    gitnook::gitnook::list(&root).unwrap();
}

// ── Test 3: Exclude hygiene ──────────────────────────────────────────────────

#[test]
fn exclude_hygiene_add_then_remove() {
    let (_tmp, root) = setup();

    gitnook::gitnook::init(&root, "default").unwrap();
    let file = make_file(&root, "secret.txt", "password");

    gitnook::gitnook::add(&root, &[file.clone()], None).unwrap();

    // After add: file is in exclude
    assert!(has_exclude(&root, "secret.txt"));

    gitnook::gitnook::remove(&root, &file, None).unwrap();

    // After remove: file is gone from exclude
    assert!(!has_exclude(&root, "secret.txt"));
    assert!(!index_has(&root, "default", "secret.txt"));
}

// ── Test 4: Active switching ─────────────────────────────────────────────────

#[test]
fn active_switching_affects_default_target() {
    let (_tmp, root) = setup();

    gitnook::gitnook::init(&root, "alpha").unwrap();
    gitnook::gitnook::init(&root, "beta").unwrap();

    // Active is "alpha" (first created)
    let file_a = make_file(&root, "a.txt", "content a");
    gitnook::gitnook::add(&root, &[file_a], None).unwrap();
    assert!(index_has(&root, "alpha", "a.txt"));
    assert!(!index_has(&root, "beta", "a.txt"));

    // Switch active to "beta"
    gitnook::gitnook::switch(&root, "beta").unwrap();
    let cfg = gitnook::config::load(&root).unwrap();
    assert_eq!(cfg.active, "beta");

    // Now add without --to goes to beta
    let file_b = make_file(&root, "b.txt", "content b");
    gitnook::gitnook::add(&root, &[file_b], None).unwrap();
    assert!(index_has(&root, "beta", "b.txt"));
    assert!(!index_has(&root, "alpha", "b.txt"));

    // list reflects the new active
    gitnook::gitnook::list(&root).unwrap();
}

// ── Test 5: Outer git isolation ──────────────────────────────────────────────

#[test]
fn outer_git_isolation_file_excluded_from_outer_git() {
    let (_tmp, root) = setup();

    gitnook::gitnook::init(&root, "default").unwrap();
    let file = make_file(&root, "private.txt", "private data");

    gitnook::gitnook::add(&root, &[file], None).unwrap();

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

    let result = gitnook::repo::find_git_root();

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
