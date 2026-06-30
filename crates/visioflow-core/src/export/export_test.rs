use std::collections::HashMap;

use super::{emit_bash, emit_ps1, vars_from_payloads};

#[test]
fn emit_bash_empty_map_returns_empty_string() {
    let vars = HashMap::new();
    assert_eq!(emit_bash(&vars), "");
}

#[test]
fn emit_ps1_empty_map_returns_empty_string() {
    let vars = HashMap::new();
    assert_eq!(emit_ps1(&vars), "");
}

#[test]
fn emit_bash_single_var() {
    let mut vars = HashMap::new();
    vars.insert("QR_VAR_ASSET".to_string(), "value".to_string());

    assert_eq!(emit_bash(&vars), "export QR_VAR_ASSET='value'\n");
}

#[test]
fn emit_ps1_single_var() {
    let mut vars = HashMap::new();
    vars.insert("QR_VAR_ASSET".to_string(), "value".to_string());

    assert_eq!(emit_ps1(&vars), "$env:QR_VAR_ASSET = 'value'\n");
}

#[test]
fn emit_bash_escapes_single_quotes() {
    let mut vars = HashMap::new();
    vars.insert("QR_RAW".to_string(), "it's a $test".to_string());

    assert_eq!(emit_bash(&vars), "export QR_RAW='it'\\''s a $test'\n");
}

#[test]
fn emit_ps1_escapes_single_quotes() {
    let mut vars = HashMap::new();
    vars.insert("QR_RAW".to_string(), "it's a $test".to_string());

    assert_eq!(emit_ps1(&vars), "$env:QR_RAW = 'it''s a $test'\n");
}

#[test]
fn emit_bash_special_characters_stay_literal() {
    let mut vars = HashMap::new();
    vars.insert("QR_RAW".to_string(), "a\"b`c\\d&e|f;g".to_string());

    let out = emit_bash(&vars);
    assert_eq!(out, "export QR_RAW='a\"b`c\\d&e|f;g'\n");
}

#[test]
fn emit_bash_sorts_keys_for_stable_output() {
    let mut vars = HashMap::new();
    vars.insert("QR_VAR_ZEBRA".to_string(), "z".to_string());
    vars.insert("QR_RAW".to_string(), "raw".to_string());
    vars.insert("QR_VAR_ALPHA".to_string(), "a".to_string());

    assert_eq!(
        emit_bash(&vars),
        "export QR_RAW='raw'\nexport QR_VAR_ALPHA='a'\nexport QR_VAR_ZEBRA='z'\n"
    );
}

#[test]
fn vars_from_payloads_single_sets_qr_raw() {
    let vars = vars_from_payloads(&["hello".to_string()]);

    assert_eq!(vars.get("QR_RAW"), Some(&"hello".to_string()));
    assert_eq!(vars.len(), 1);
}

#[test]
fn vars_from_payloads_empty_is_empty_map() {
    let vars = vars_from_payloads(&[]);
    assert!(vars.is_empty());
}

#[test]
fn vars_from_payloads_multiple_joins_with_newline() {
    let vars = vars_from_payloads(&["one".to_string(), "two".to_string()]);

    assert_eq!(vars.get("QR_RAW"), Some(&"one\ntwo".to_string()));
}
