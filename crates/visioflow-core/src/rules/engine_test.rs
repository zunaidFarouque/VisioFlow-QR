use crate::rules::{
    apply_rule, merge_native_vars, resolve_payload_fully, PayloadRouter, ResolvedVars, Rule,
    RuleEngine, RuleError,
};

#[test]
fn apply_rule_always_sets_qr_raw() {
    let rule = Rule::new("plain");
    let resolved = apply_rule(&rule, "hello-world").expect("route should succeed");

    assert_eq!(resolved.raw(), Some("hello-world"));
    assert_eq!(resolved.get("QR_VAR_ASSET"), None);
}

#[test]
fn apply_rule_maps_named_capture_to_qr_var_default() {
    let mut rule = Rule::new("asset");
    rule.regex = Some(r"ASSET:(?P<asset>\d+)".to_owned());

    let resolved = apply_rule(&rule, "ASSET:42").expect("regex should match");

    assert_eq!(resolved.raw(), Some("ASSET:42"));
    assert_eq!(resolved.get("QR_VAR_ASSET"), Some("42"));
}

#[test]
fn apply_rule_uses_custom_capture_mapping() {
    let mut rule = Rule::new("custom");
    rule.regex = Some(r"id:(?P<id>\w+)".to_owned());
    rule.captures
        .insert("id".to_owned(), "IDENTIFIER".to_owned());

    let resolved = apply_rule(&rule, "id:abc123").expect("regex should match");

    assert_eq!(resolved.get("QR_VAR_IDENTIFIER"), Some("abc123"));
    assert_eq!(resolved.get("QR_VAR_ID"), None);
}

#[test]
fn apply_rule_returns_no_match_when_regex_fails() {
    let mut rule = Rule::new("strict");
    rule.regex = Some(r"^PREFIX:(?P<code>\d+)$".to_owned());

    let err = apply_rule(&rule, "wrong").expect_err("should not match");
    assert_eq!(err, RuleError::NoMatch);
}

#[test]
fn apply_rule_returns_invalid_regex_error() {
    let mut rule = Rule::new("bad");
    rule.regex = Some(r"(?P<unclosed".to_owned());

    let err = apply_rule(&rule, "payload").expect_err("invalid regex");
    assert!(matches!(err, RuleError::InvalidRegex(_)));
}

#[test]
fn rule_engine_routes_via_store() {
    let mut store = crate::rules::store::MockRuleStore::new();
    let mut rule = Rule::new("wifi");
    rule.regex = Some(r"WIFI:(?P<ssid>[^;]+)".to_owned());

    store
        .expect_get()
        .with(mockall::predicate::eq("wifi"))
        .times(1)
        .returning(move |_| Ok(rule.clone()));

    let engine = RuleEngine::new(store);
    let resolved = engine
        .route("wifi", "WIFI:corp-net")
        .expect("engine route should succeed");

    assert_eq!(resolved.get("QR_VAR_SSID"), Some("corp-net"));
}

#[test]
fn rule_engine_propagates_not_found() {
    let mut store = crate::rules::store::MockRuleStore::new();
    store
        .expect_get()
        .with(mockall::predicate::eq("missing"))
        .times(1)
        .returning(|_| Err(RuleError::NotFound("missing".to_owned())));

    let engine = RuleEngine::new(store);
    let err = engine
        .route("missing", "payload")
        .expect_err("missing rule");
    assert_eq!(err, RuleError::NotFound("missing".to_owned()));
}

#[test]
fn apply_rule_without_regex_sets_only_qr_raw() {
    let rule = Rule::new("no-regex");
    let resolved = apply_rule(&rule, "any-payload").expect("should succeed");

    assert_eq!(resolved.raw(), Some("any-payload"));
    assert_eq!(resolved.into_inner().len(), 1);
}

#[test]
fn apply_rule_multiple_named_captures() {
    let mut rule = Rule::new("multi");
    rule.regex = Some(r"(?P<site>[A-Z]+)-(?P<num>\d+)".to_owned());

    let resolved = apply_rule(&rule, "WAREHOUSE-99").expect("should match");

    assert_eq!(resolved.get("QR_VAR_SITE"), Some("WAREHOUSE"));
    assert_eq!(resolved.get("QR_VAR_NUM"), Some("99"));
}

#[test]
fn merge_native_vars_adds_wifi_keys() {
    let mut resolved = ResolvedVars::new();
    resolved.insert(ResolvedVars::QR_RAW, "WIFI:T:WPA;S:corp;P:secret;;");

    merge_native_vars(&mut resolved, "WIFI:T:WPA;S:corp;P:secret;;");

    assert_eq!(resolved.get("QR_NATIVE_WIFI_SSID"), Some("corp"));
    assert_eq!(resolved.get("QR_NATIVE_WIFI_PASSWORD"), Some("secret"));
}

#[test]
fn merge_native_vars_adds_uri_keys() {
    let mut resolved = ResolvedVars::new();
    let payload = "https://example.com:8080/path";

    merge_native_vars(&mut resolved, payload);

    assert_eq!(resolved.get("QR_NATIVE_URI_SCHEME"), Some("https"));
    assert_eq!(resolved.get("QR_NATIVE_URI_HOST"), Some("example.com"));
    assert_eq!(resolved.get("QR_NATIVE_URI_PORT"), Some("8080"));
    assert_eq!(resolved.get("QR_NATIVE_URI_PATH"), Some("/path"));
}

#[test]
fn resolve_payload_fully_merges_rule_and_native_vars() {
    let rule = Rule::new("wifi");
    let payload = "WIFI:T:WPA;S:guest;P:pass;;";

    let resolved = resolve_payload_fully(&rule, payload).expect("resolve");

    assert_eq!(resolved.raw(), Some(payload));
    assert_eq!(resolved.get("QR_NATIVE_WIFI_SSID"), Some("guest"));
    assert_eq!(resolved.get("QR_NATIVE_WIFI_PASSWORD"), Some("pass"));
}

#[test]
fn resolve_payload_fully_applies_regex_and_native() {
    let mut rule = Rule::new("asset");
    rule.regex = Some(r"ASSET:(?P<asset>\d+)".to_owned());

    let resolved = resolve_payload_fully(&rule, "ASSET:42").expect("match");

    assert_eq!(resolved.get("QR_VAR_ASSET"), Some("42"));
}

#[test]
fn resolve_payload_fully_propagates_regex_no_match() {
    let mut rule = Rule::new("strict");
    rule.regex = Some(r"^ASSET:(?P<asset>\d+)$".to_owned());

    let err = resolve_payload_fully(&rule, "WIFI:T:WPA;S:x;P:y;;").expect_err("no match");
    assert_eq!(err, RuleError::NoMatch);
}

#[test]
fn rule_engine_route_fully_loads_rule_and_native_vars() {
    let mut store = crate::rules::store::MockRuleStore::new();
    let rule = Rule::new("uri");

    store
        .expect_get()
        .with(mockall::predicate::eq("uri"))
        .times(1)
        .returning(move |_| Ok(rule.clone()));

    let engine = RuleEngine::new(store);
    let routed = engine
        .route_fully("uri", "https://host.test/path")
        .expect("route fully");

    assert_eq!(routed.rule.name, "uri");
    assert_eq!(routed.vars.get("QR_NATIVE_URI_HOST"), Some("host.test"));
}
