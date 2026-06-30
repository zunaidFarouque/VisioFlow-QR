use std::collections::BTreeMap;

use super::handler::DaemonHandler;
use super::{ClientMessage, ServerMessage};
use crate::rules::{FileRuleStore, Rule, RuleStore};
use tempfile::TempDir;

fn temp_handler() -> (TempDir, DaemonHandler) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rules.json");
    let store = FileRuleStore::new(path);
    let handler = DaemonHandler::new(store).expect("handler");
    (dir, handler)
}

#[test]
fn handler_ping_returns_pong() {
    let (_dir, mut handler) = temp_handler();
    let response = handler.handle(ClientMessage::Ping { id: 1 }, false);
    assert_eq!(response, ServerMessage::Pong { id: 1 });
}

#[test]
fn handler_list_rules_returns_names() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rules.json");
    let store = FileRuleStore::new(path);
    let mut rules = BTreeMap::new();
    rules.insert("wifi".to_owned(), Rule::new("wifi"));
    rules.insert("uri".to_owned(), Rule::new("uri"));
    store.save_all(&rules).expect("save");

    let mut handler = DaemonHandler::new(store).expect("handler");
    let response = handler.handle(ClientMessage::ListRules { id: 2 }, false);
    match response {
        ServerMessage::RulesList { id, mut names } => {
            assert_eq!(id, 2);
            names.sort();
            assert_eq!(names, vec!["uri", "wifi"]);
        }
        other => panic!("expected RulesList, got {other:?}"),
    }
}

#[test]
fn handler_execute_rule_resolves_vars() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rules.json");
    let store = FileRuleStore::new(path);
    let mut rules = BTreeMap::new();
    let mut rule = Rule::new("asset");
    rule.regex = Some(r"ASSET:(?P<asset>\d+)".to_owned());
    rules.insert("asset".to_owned(), rule);
    store.save_all(&rules).expect("save");

    let mut handler = DaemonHandler::new(store).expect("handler");
    let response = handler.handle(
        ClientMessage::ExecuteRule {
            id: 3,
            name: "asset".into(),
            payload: "ASSET:42".into(),
        },
        false,
    );

    match response {
        ServerMessage::RuleResult { id, vars, exit_code } => {
            assert_eq!(id, 3);
            assert_eq!(vars.get("QR_RAW").map(String::as_str), Some("ASSET:42"));
            assert_eq!(vars.get("QR_VAR_ASSET").map(String::as_str), Some("42"));
            assert!(exit_code.is_none());
        }
        other => panic!("expected RuleResult, got {other:?}"),
    }
}

#[test]
fn handler_execute_rule_merges_wifi_native_vars() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rules.json");
    let store = FileRuleStore::new(path);
    let mut rules = BTreeMap::new();
    rules.insert("wifi".to_owned(), Rule::new("wifi"));
    store.save_all(&rules).expect("save");

    let mut handler = DaemonHandler::new(store).expect("handler");
    let response = handler.handle(
        ClientMessage::ExecuteRule {
            id: 4,
            name: "wifi".into(),
            payload: "WIFI:T:WPA;S:MyNet;P:secret;;".into(),
        },
        false,
    );

    match response {
        ServerMessage::RuleResult { vars, .. } => {
            assert_eq!(
                vars.get("QR_NATIVE_WIFI_SSID").map(String::as_str),
                Some("MyNet")
            );
            assert!(vars.contains_key("QR_NATIVE_WIFI_PASSWORD"));
        }
        other => panic!("expected RuleResult, got {other:?}"),
    }
}

#[test]
fn handler_execute_missing_rule_returns_error() {
    let (_dir, mut handler) = temp_handler();
    let response = handler.handle(
        ClientMessage::ExecuteRule {
            id: 5,
            name: "missing".into(),
            payload: "x".into(),
        },
        false,
    );
    match response {
        ServerMessage::Error { id, message } => {
            assert_eq!(id, 5);
            assert!(message.contains("rule not found"));
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

#[test]
fn handler_execute_wifi_connect_missing_ssid_returns_error() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rules.json");
    let store = FileRuleStore::new(path);
    let mut rules = BTreeMap::new();
    let mut rule = Rule::new("wifi");
    rule.wifi_connect = true;
    rules.insert("wifi".to_owned(), rule);
    store.save_all(&rules).expect("save");

    let mut handler = DaemonHandler::new(store).expect("handler");
    let response = handler.handle(
        ClientMessage::ExecuteRule {
            id: 8,
            name: "wifi".into(),
            payload: "not-a-wifi-payload".into(),
        },
        false,
    );

    match response {
        ServerMessage::Error { id, message } => {
            assert_eq!(id, 8);
            assert!(message.contains("wifi connect failed"));
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

#[test]
fn handler_reload_refreshes_rules_from_disk() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rules.json");
    let store = FileRuleStore::new(path);
    let handler = DaemonHandler::new(store.clone()).expect("handler");

    let mut rules = BTreeMap::new();
    rules.insert("new_rule".to_owned(), Rule::new("new_rule"));
    store.save_all(&rules).expect("save");

    let mut handler = handler;
    let response = handler.handle(ClientMessage::Reload { id: 6 }, false);
    assert_eq!(response, ServerMessage::Pong { id: 6 });

    let list = handler.handle(ClientMessage::ListRules { id: 7 }, false);
    match list {
        ServerMessage::RulesList { names, .. } => {
            assert_eq!(names, vec!["new_rule".to_owned()]);
        }
        other => panic!("expected RulesList, got {other:?}"),
    }
}
