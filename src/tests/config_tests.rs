use super::*;
use std::collections::HashMap;
use tempfile::TempDir;

fn make_config(active: &str, names: &[&str]) -> GitnookConfig {
    let mut cfg = GitnookConfig {
        active: active.to_string(),
        gitnooks: HashMap::new(),
    };
    for &name in names {
        cfg.gitnooks.insert(
            name.to_string(),
            GitnookEntry {
                created: "2025-01-01T00:00:00Z".to_string(),
            },
        );
    }
    cfg
}

#[test]
fn missing_file_returns_descriptive_error() {
    let tmp = TempDir::new().unwrap();
    let err = load(tmp.path()).unwrap_err();
    assert!(
        err.to_string().contains("gitnook init"),
        "expected hint to run 'gitnook init', got: {err}"
    );
}

#[test]
fn save_creates_file_and_load_reads_it() {
    let tmp = TempDir::new().unwrap();
    let cfg = make_config("default", &["default"]);
    save(tmp.path(), &cfg).unwrap();

    let path = tmp.path().join(".gitnook").join("config.toml");
    assert!(path.exists(), "config.toml was not created");
}

#[test]
fn round_trip_preserves_fields() {
    let tmp = TempDir::new().unwrap();
    let cfg = make_config("secrets", &["secrets", "personal"]);
    save(tmp.path(), &cfg).unwrap();

    let loaded = load(tmp.path()).unwrap();
    assert_eq!(loaded.active, "secrets");
    assert!(loaded.gitnooks.contains_key("secrets"));
    assert!(loaded.gitnooks.contains_key("personal"));
    assert_eq!(loaded.gitnooks["secrets"].created, "2025-01-01T00:00:00Z");
}

#[test]
fn set_active_updates_active_field() {
    let tmp = TempDir::new().unwrap();
    save(tmp.path(), &make_config("default", &["default", "work"])).unwrap();
    set_active(tmp.path(), "work").unwrap();
    assert_eq!(load(tmp.path()).unwrap().active, "work");
}

#[test]
fn save_is_atomic_temp_file_removed() {
    let tmp = TempDir::new().unwrap();
    let cfg = make_config("default", &["default"]);
    save(tmp.path(), &cfg).unwrap();

    let tmp_path = tmp.path().join(".gitnook").join("config.toml.tmp");
    assert!(!tmp_path.exists(), "temp file should be removed after save");
}
