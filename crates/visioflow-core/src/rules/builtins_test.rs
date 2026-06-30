use crate::rules::{
    is_builtin_trigger, is_excluded_from_auto_scan, is_reserved_rule_name, RESERVED_RULE_NAMES,
};

#[test]
fn reserved_rule_names_contains_copy_plain_auto() {
    assert!(RESERVED_RULE_NAMES.contains(&"copy"));
    assert!(RESERVED_RULE_NAMES.contains(&"plain"));
    assert!(RESERVED_RULE_NAMES.contains(&"auto"));
}

#[test]
fn is_reserved_rule_name_is_case_sensitive() {
    assert!(is_reserved_rule_name("copy"));
    assert!(!is_reserved_rule_name("Copy"));
    assert!(!is_reserved_rule_name("url"));
}

#[test]
fn is_excluded_from_auto_scan_allows_plain_catch_all_rule() {
    assert!(is_excluded_from_auto_scan("copy"));
    assert!(is_excluded_from_auto_scan("auto"));
    assert!(!is_excluded_from_auto_scan("plain"));
    assert!(!is_excluded_from_auto_scan("url"));
}

#[test]
fn is_builtin_trigger_includes_copy_and_plain_not_auto() {
    assert!(is_builtin_trigger("copy"));
    assert!(is_builtin_trigger("plain"));
    assert!(!is_builtin_trigger("auto"));
    assert!(!is_builtin_trigger("url"));
}
