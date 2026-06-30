use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::rules::{apply_rule, FileRuleStore, Rule, RuleError, RuleStore};

fn temp_rules_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "visioflow-rules-test-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    path
}

#[test]
fn file_store_round_trip_persistence() {
    let path = temp_rules_path();
    let store = FileRuleStore::new(path.clone());

    let mut rule = Rule::new("ship");
    rule.regex = Some(r"TRACK:(?P<track>\d+)".to_owned());
    rule.captures
        .insert("track".to_owned(), "TRACKING".to_owned());
    rule.exec = Some(PathBuf::from("/usr/bin/notify.sh"));

    store.upsert(&rule).expect("upsert should succeed");

    let loaded = store.get("ship").expect("get should succeed");
    assert_eq!(loaded, rule);

    let routed = apply_rule(&loaded, "TRACK:12345").expect("apply should match");
    assert_eq!(routed.get("QR_VAR_TRACKING"), Some("12345"));
    assert_eq!(routed.raw(), Some("TRACK:12345"));

    let all = store.load_all().expect("load_all should succeed");
    assert_eq!(all.len(), 1);
    assert!(all.contains_key("ship"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn file_store_get_missing_rule_returns_not_found() {
    let path = temp_rules_path();
    let store = FileRuleStore::new(path.clone());

    let err = store.get("nope").expect_err("rule should not exist");
    assert_eq!(err, RuleError::NotFound("nope".to_owned()));

    let _ = std::fs::remove_file(path);
}

#[test]
fn file_store_delete_removes_rule() {
    let path = temp_rules_path();
    let store = FileRuleStore::new(path.clone());

    let rule = Rule::new("temp");
    store.upsert(&rule).expect("upsert");
    store.delete("temp").expect("delete");

    let err = store.get("temp").expect_err("deleted rule");
    assert_eq!(err, RuleError::NotFound("temp".to_owned()));

    let _ = std::fs::remove_file(path);
}

#[test]
fn file_store_save_all_replaces_collection() {
    let path = temp_rules_path();
    let store = FileRuleStore::new(path.clone());

    let mut rules = BTreeMap::new();
    rules.insert("a".to_owned(), Rule::new("a"));
    rules.insert("b".to_owned(), Rule::new("b"));

    store.save_all(&rules).expect("save_all");

    let loaded = store.load_all().expect("load_all");
    assert_eq!(loaded.len(), 2);

    let _ = std::fs::remove_file(path);
}

#[test]
fn file_store_empty_file_reads_as_empty() {
    let path = temp_rules_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, "").expect("write empty file");

    let store = FileRuleStore::new(path.clone());
    let rules = store.load_all().expect("empty file should load");
    assert!(rules.is_empty());

    let _ = std::fs::remove_file(path);
}

#[test]
fn apply_rule_no_match_after_round_trip() {
    let path = temp_rules_path();
    let store = FileRuleStore::new(path.clone());

    let mut rule = Rule::new("gate");
    rule.regex = Some(r"^OK:(?P<code>\d+)$".to_owned());
    store.upsert(&rule).expect("upsert");

    let loaded = store.get("gate").expect("get");
    let err = apply_rule(&loaded, "FAIL:1").expect_err("no match");
    assert_eq!(err, RuleError::NoMatch);

    let _ = std::fs::remove_file(path);
}

#[test]
fn default_path_is_under_config_dir() {
    let path = FileRuleStore::default_path().expect("config dir available");
    let path_str = path.to_string_lossy();
    assert!(path_str.contains("visioflow"));
    assert!(path_str.ends_with("rules.json"));
}
