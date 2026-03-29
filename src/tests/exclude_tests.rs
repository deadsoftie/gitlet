use super::*;
use std::fs;
use tempfile::TempDir;

/// Create a fake `.git/info/` directory so helpers have somewhere to write.
fn setup_git_dir(tmp: &TempDir) {
    fs::create_dir_all(tmp.path().join(".git").join("info")).unwrap();
}

#[test]
fn has_exclusion_false_when_file_absent() {
    let tmp = TempDir::new().unwrap();
    setup_git_dir(&tmp);
    assert!(!has_exclusion(tmp.path(), "notes.md").unwrap());
}

#[test]
fn add_creates_file_with_header_and_pattern() {
    let tmp = TempDir::new().unwrap();
    setup_git_dir(&tmp);

    add_exclusion(tmp.path(), "notes.md").unwrap();

    let contents =
        fs::read_to_string(tmp.path().join(".git").join("info").join("exclude")).unwrap();
    assert!(contents.contains("# gitnook managed entries"));
    assert!(contents.contains("notes.md"));
}

#[test]
fn add_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    setup_git_dir(&tmp);

    add_exclusion(tmp.path(), "notes.md").unwrap();
    add_exclusion(tmp.path(), "notes.md").unwrap();

    let contents =
        fs::read_to_string(tmp.path().join(".git").join("info").join("exclude")).unwrap();
    assert_eq!(contents.matches("notes.md").count(), 1);
}

#[test]
fn has_exclusion_true_after_add() {
    let tmp = TempDir::new().unwrap();
    setup_git_dir(&tmp);

    add_exclusion(tmp.path(), ".env.local").unwrap();
    assert!(has_exclusion(tmp.path(), ".env.local").unwrap());
}

#[test]
fn remove_deletes_pattern_line() {
    let tmp = TempDir::new().unwrap();
    setup_git_dir(&tmp);

    add_exclusion(tmp.path(), "notes.md").unwrap();
    add_exclusion(tmp.path(), ".env.local").unwrap();
    remove_exclusion(tmp.path(), "notes.md").unwrap();

    assert!(!has_exclusion(tmp.path(), "notes.md").unwrap());
    assert!(has_exclusion(tmp.path(), ".env.local").unwrap());
}

#[test]
fn remove_is_noop_when_file_absent() {
    let tmp = TempDir::new().unwrap();
    setup_git_dir(&tmp);
    remove_exclusion(tmp.path(), "notes.md").unwrap();
}
