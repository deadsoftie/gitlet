use super::*;
use std::fs;
use tempfile::TempDir;

// ── helpers ────────────────────────────────────────────────────────────────

/// Create a temp dir with a real outer git repo inside it.
/// Returns (TempDir, canonical_root).  TempDir must be kept alive.
fn setup() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    git2::Repository::init(tmp.path()).unwrap();
    let root = tmp.path().canonicalize().unwrap();
    (tmp, root)
}

/// Write a file at `<root>/<name>` and return its absolute path as a String.
fn make_file(root: &Path, name: &str, content: &str) -> String {
    let path = root.join(name);
    fs::write(&path, content).unwrap();
    path.to_string_lossy().into_owned()
}

/// Return the number of entries in a gitnook's index.
fn index_len(root: &Path, name: &str) -> usize {
    let repo = git2::Repository::open(root.join(".gitnook").join(name)).unwrap();
    repo.index().unwrap().len()
}

/// Return true if the gitnook's index contains an entry with the given relative path.
fn index_has(root: &Path, gitnook: &str, rel: &str) -> bool {
    let repo = git2::Repository::open(root.join(".gitnook").join(gitnook)).unwrap();
    let index = repo.index().unwrap();
    index.get_path(Path::new(rel), 0).is_some()
}

// ── normalize_path ─────────────────────────────────────────────────────────

#[test]
fn normalize_strips_cur_dir() {
    let p = PathBuf::from("/a/./b/./c");
    assert_eq!(normalize_path(&p), PathBuf::from("/a/b/c"));
}

#[test]
fn normalize_resolves_parent_dir() {
    let p = PathBuf::from("/a/b/../c");
    assert_eq!(normalize_path(&p), PathBuf::from("/a/c"));
}

#[test]
fn normalize_handles_nested_parent() {
    let p = PathBuf::from("/a/b/c/../../d");
    assert_eq!(normalize_path(&p), PathBuf::from("/a/d"));
}

#[test]
fn normalize_identity_on_clean_path() {
    let p = PathBuf::from("/a/b/c");
    assert_eq!(normalize_path(&p), PathBuf::from("/a/b/c"));
}

// ── init ───────────────────────────────────────────────────────────────────

#[test]
fn init_creates_bare_repo_directory() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    assert!(root.join(".gitnook/default/HEAD").exists());
}

#[test]
fn init_creates_config_with_active() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let cfg = crate::config::load(&root).unwrap();
    assert_eq!(cfg.active, "default");
    assert!(cfg.gitnooks.contains_key("default"));
}

#[test]
fn init_first_gitnook_becomes_active() {
    let (_tmp, root) = setup();
    init(&root, "first").unwrap();
    init(&root, "second").unwrap();
    let cfg = crate::config::load(&root).unwrap();
    assert_eq!(cfg.active, "first");
}

#[test]
fn init_adds_gitnook_dir_to_exclude() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    assert!(crate::exclude::has_exclusion(&root, ".gitnook/").unwrap());
}

#[test]
fn init_duplicate_name_errors() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let err = init(&root, "default").unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn init_multiple_names_all_registered_in_config() {
    let (_tmp, root) = setup();
    init(&root, "alpha").unwrap();
    init(&root, "beta").unwrap();
    let cfg = crate::config::load(&root).unwrap();
    assert!(cfg.gitnooks.contains_key("alpha"));
    assert!(cfg.gitnooks.contains_key("beta"));
}

// ── add ────────────────────────────────────────────────────────────────────

#[test]
fn add_stages_file_in_gitnook_index() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file], None).unwrap();
    assert!(index_has(&root, "default", "notes.md"));
}

#[test]
fn add_writes_path_to_exclude() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file], None).unwrap();
    assert!(crate::exclude::has_exclusion(&root, "notes.md").unwrap());
}

#[test]
fn add_multiple_files_in_one_call() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let f1 = make_file(&root, "a.md", "a");
    let f2 = make_file(&root, "b.md", "b");
    add(&root, &[f1, f2], None).unwrap();
    assert_eq!(index_len(&root, "default"), 2);
}

#[test]
fn add_to_named_gitnook_via_to_flag() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    init(&root, "secrets").unwrap();
    let file = make_file(&root, ".env", "pw=x");
    add(&root, &[file], Some("secrets")).unwrap();
    assert!(index_has(&root, "secrets", ".env"));
    assert_eq!(index_len(&root, "default"), 0);
}

#[test]
fn add_cross_gitnook_errors() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    init(&root, "other").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file.clone()], Some("default")).unwrap();
    let err = add(&root, &[file], Some("other")).unwrap_err();
    assert!(err.to_string().contains("already tracked by gitnook"));
}

#[test]
fn add_restage_same_gitnook_updates_index() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "v1");
    add(&root, &[file.clone()], None).unwrap();
    fs::write(&file, "v2").unwrap();
    // Re-adding to same gitnook stages the modification — should not error
    add(&root, &[file], None).unwrap();
    assert_eq!(index_len(&root, "default"), 1);
}

#[test]
fn add_nonexistent_file_errors() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let bad = root.join("ghost.md").to_string_lossy().into_owned();
    let err = add(&root, &[bad], None).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

// ── remove ─────────────────────────────────────────────────────────────────

#[test]
fn remove_clears_index_entry() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file.clone()], None).unwrap();
    remove(&root, &file, None).unwrap();
    assert!(!index_has(&root, "default", "notes.md"));
}

#[test]
fn remove_clears_exclude_entry() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file.clone()], None).unwrap();
    remove(&root, &file, None).unwrap();
    assert!(!crate::exclude::has_exclusion(&root, "notes.md").unwrap());
}

#[test]
fn remove_untracked_file_errors() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    let err = remove(&root, &file, None).unwrap_err();
    assert!(err.to_string().contains("not tracked"));
}

#[test]
fn remove_wrong_gitnook_errors() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    init(&root, "other").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file.clone()], Some("default")).unwrap();
    // Trying to remove from "other" when it belongs to "default"
    let err = remove(&root, &file, Some("other")).unwrap_err();
    assert!(err.to_string().contains("not tracked"));
}

// ── commit ─────────────────────────────────────────────────────────────────

#[test]
fn commit_creates_root_commit() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file], None).unwrap();
    commit(&root, "initial", None).unwrap();

    let repo = git2::Repository::open(root.join(".gitnook/default")).unwrap();
    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head_commit.message().unwrap(), "initial");
    assert_eq!(head_commit.parent_count(), 0);
}

#[test]
fn commit_chains_parent() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "v1");
    add(&root, &[file.clone()], None).unwrap();
    commit(&root, "first", None).unwrap();
    fs::write(&file, "v2").unwrap();
    add(&root, &[file], None).unwrap();
    commit(&root, "second", None).unwrap();

    let repo = git2::Repository::open(root.join(".gitnook/default")).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.message().unwrap(), "second");
    assert_eq!(head.parent_count(), 1);
    assert_eq!(head.parent(0).unwrap().message().unwrap(), "first");
}

#[test]
fn commit_uses_identity_fallback() {
    // Repo with no user.name/email configured — should use fallback and not error
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file], None).unwrap();
    commit(&root, "test commit", None).unwrap();

    let repo = git2::Repository::open(root.join(".gitnook/default")).unwrap();
    let c = repo.head().unwrap().peel_to_commit().unwrap();
    // Either real config or the fallback values — just must not be empty
    assert!(!c.author().name().unwrap_or("").is_empty());
}

// ── status ─────────────────────────────────────────────────────────────────

#[test]
fn status_new_file_before_commit() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file], None).unwrap();
    let summary = gitnook_status_summary(&root, "default").unwrap();
    assert!(summary.contains("new file"));
}

#[test]
fn status_clean_after_commit() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file], None).unwrap();
    commit(&root, "init", None).unwrap();
    let summary = gitnook_status_summary(&root, "default").unwrap();
    assert_eq!(summary, "clean");
}

#[test]
fn status_modified_after_disk_change() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let path = root.join("notes.md");
    fs::write(&path, "v1").unwrap();
    add(&root, &[path.to_string_lossy().into_owned()], None).unwrap();
    commit(&root, "init", None).unwrap();

    fs::write(&path, "v2").unwrap();
    let summary = gitnook_status_summary(&root, "default").unwrap();
    assert!(summary.contains("modified"));
}

#[test]
fn status_no_gitnooks_prints_message() {
    let (_tmp, root) = setup();
    status(&root, None).unwrap();
}

#[test]
fn status_unknown_name_errors() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let err = status(&root, Some("nonexistent")).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

// ── log ────────────────────────────────────────────────────────────────────

#[test]
fn log_empty_gitnook_returns_ok() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    log(&root, None).unwrap();
}

#[test]
fn log_after_commits_returns_ok() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file], None).unwrap();
    commit(&root, "first commit", None).unwrap();
    log(&root, None).unwrap();
}

#[test]
fn log_unknown_name_errors() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let err = log(&root, Some("ghost")).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

// ── list ───────────────────────────────────────────────────────────────────

#[test]
fn list_no_gitnooks_returns_ok() {
    let (_tmp, root) = setup();
    list(&root).unwrap();
}

#[test]
fn list_shows_correct_file_counts() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let f1 = make_file(&root, "a.md", "a");
    let f2 = make_file(&root, "b.md", "b");
    add(&root, &[f1, f2], None).unwrap();
    assert_eq!(index_len(&root, "default"), 2);
    list(&root).unwrap();
}

// ── switch ──────────────────────────────────────────────────────────────────

#[test]
fn switch_changes_active_gitnook() {
    let (_tmp, root) = setup();
    init(&root, "first").unwrap();
    init(&root, "second").unwrap();
    let cfg = crate::config::load(&root).unwrap();
    assert_eq!(cfg.active, "first");

    switch(&root, "second").unwrap();
    let cfg = crate::config::load(&root).unwrap();
    assert_eq!(cfg.active, "second");
}

#[test]
fn switch_unknown_name_errors() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let err = switch(&root, "nonexistent").unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn switch_reflected_in_list() {
    let (_tmp, root) = setup();
    init(&root, "alpha").unwrap();
    init(&root, "beta").unwrap();
    switch(&root, "beta").unwrap();
    let cfg = crate::config::load(&root).unwrap();
    assert_eq!(cfg.active, "beta");
    list(&root).unwrap();
}

// ── diff ────────────────────────────────────────────────────────────────────

#[test]
fn diff_no_changes_after_commit() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello\n");
    add(&root, &[file], None).unwrap();
    commit(&root, "init", None).unwrap();
    // File unchanged — diff should report no changes without error
    diff(&root, None).unwrap();
}

#[test]
fn diff_shows_new_file_before_commit() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    make_file(&root, "notes.md", "hello\n");
    add(&root, &[root.join("notes.md").to_string_lossy().into_owned()], None).unwrap();
    // No commits yet — staged file should appear as new
    diff(&root, None).unwrap();
}

#[test]
fn diff_shows_modification_after_commit() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let path = root.join("notes.md");
    fs::write(&path, "v1\n").unwrap();
    add(&root, &[path.to_string_lossy().into_owned()], None).unwrap();
    commit(&root, "init", None).unwrap();

    fs::write(&path, "v2\n").unwrap();
    // File modified on disk — diff should complete without error
    diff(&root, None).unwrap();
}

#[test]
fn diff_unknown_name_errors() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let err = diff(&root, Some("ghost")).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn diff_targets_named_gitnook() {
    let (_tmp, root) = setup();
    init(&root, "first").unwrap();
    init(&root, "second").unwrap();
    let file = make_file(&root, "notes.md", "hello\n");
    add(&root, &[file], Some("second")).unwrap();
    diff(&root, Some("second")).unwrap();
}

// ── destroy ─────────────────────────────────────────────────────────────────

#[test]
fn destroy_removes_gitnook_directory() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    destroy(&root, "default").unwrap();
    assert!(!root.join(".gitnook/default").exists());
}

#[test]
fn destroy_removes_tracked_files_from_exclude() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let file = make_file(&root, "notes.md", "hello");
    add(&root, &[file], None).unwrap();
    assert!(crate::exclude::has_exclusion(&root, "notes.md").unwrap());

    destroy(&root, "default").unwrap();
    assert!(!crate::exclude::has_exclusion(&root, "notes.md").unwrap());
}

#[test]
fn destroy_last_gitnook_removes_gitnook_root_and_exclude_entry() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    assert!(crate::exclude::has_exclusion(&root, ".gitnook/").unwrap());

    destroy(&root, "default").unwrap();
    assert!(!root.join(".gitnook").exists());
    assert!(!crate::exclude::has_exclusion(&root, ".gitnook/").unwrap());
}

#[test]
fn destroy_one_of_two_updates_config() {
    let (_tmp, root) = setup();
    init(&root, "alpha").unwrap();
    init(&root, "beta").unwrap();
    destroy(&root, "beta").unwrap();

    let cfg = crate::config::load(&root).unwrap();
    assert!(!cfg.gitnooks.contains_key("beta"));
    assert!(cfg.gitnooks.contains_key("alpha"));
}

#[test]
fn destroy_active_gitnook_switches_active_to_remaining() {
    let (_tmp, root) = setup();
    init(&root, "first").unwrap();
    init(&root, "second").unwrap();
    // first is active; destroy it
    destroy(&root, "first").unwrap();

    let cfg = crate::config::load(&root).unwrap();
    assert!(!cfg.gitnooks.contains_key("first"));
    assert_ne!(cfg.active, "first");
}

#[test]
fn destroy_nonexistent_errors() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let err = destroy(&root, "ghost").unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn destroy_clears_multiple_tracked_files_from_exclude() {
    let (_tmp, root) = setup();
    init(&root, "default").unwrap();
    let f1 = make_file(&root, "a.txt", "a");
    let f2 = make_file(&root, "b.txt", "b");
    add(&root, &[f1, f2], None).unwrap();
    destroy(&root, "default").unwrap();
    assert!(!crate::exclude::has_exclusion(&root, "a.txt").unwrap());
    assert!(!crate::exclude::has_exclusion(&root, "b.txt").unwrap());
}
