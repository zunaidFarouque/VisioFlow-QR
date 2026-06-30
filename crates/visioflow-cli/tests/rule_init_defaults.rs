use assert_cmd::Command;
use predicates::prelude::*;
use std::collections::BTreeMap;
use visioflow_core::{resolve_share_path, Rule};

fn store_path(dir: &tempfile::TempDir) -> String {
    dir.path().join("rules.json").display().to_string()
}

const STOCK_RULE_NAMES: &[&str] = &[
    "wifi", "url", "mailto", "tel", "geo", "vcard", "clipboard", "asset", "plain",
];

#[test]
fn rule_init_defaults_installs_all_stock_rules() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(&dir);

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store, "init-defaults"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(dir.path().join("rules.json")).expect("read store");
    let rules: BTreeMap<String, Rule> = serde_json::from_str(&contents).expect("parse rules");

    for name in STOCK_RULE_NAMES {
        assert!(rules.contains_key(*name), "missing stock rule {name}");
    }

    let url = rules.get("url").expect("url");
    assert_eq!(url.priority, 10);
    assert!(url.auto_compatible);
    let exec = url.exec.as_ref().expect("url exec");
    assert_eq!(exec, &resolve_share_path("share/actions/open-url"));
}

#[test]
fn rule_init_defaults_merge_preserves_existing_rules() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(&dir);

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store, "create", "url"])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "rule",
            "--store",
            &store,
            "config",
            "url",
            "--regex",
            "^custom$",
        ])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store, "init-defaults", "--merge"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(dir.path().join("rules.json")).expect("read store");
    let rules: BTreeMap<String, Rule> = serde_json::from_str(&contents).expect("parse rules");
    assert_eq!(rules.get("url").and_then(|r| r.regex.as_deref()), Some("^custom$"));
    assert!(rules.contains_key("wifi"));
}

#[test]
fn rule_init_defaults_force_replaces_custom_rules() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(&dir);

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store, "create", "custom-only"])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store, "init-defaults", "--force"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(dir.path().join("rules.json")).expect("read store");
    let rules: BTreeMap<String, Rule> = serde_json::from_str(&contents).expect("parse rules");
    assert!(!rules.contains_key("custom-only"));
    assert!(rules.contains_key("url"));
}

#[test]
fn rule_create_rejects_reserved_builtin_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(&dir);

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store, "create", "copy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("reserved"));
}

#[test]
fn resolve_share_path_points_at_repo_action_scripts() {
    let path = resolve_share_path("share/actions/copy-text");
    assert!(path.is_file(), "expected script at {}", path.display());
}
