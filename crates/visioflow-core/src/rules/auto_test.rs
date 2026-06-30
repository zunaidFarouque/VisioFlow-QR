use std::collections::{BTreeMap, HashSet};

use crate::rules::{route_payload, AutoRouteOptions, RouteMode, RoutingEvent, Rule, RuleError};

fn mock_store(rules: Vec<Rule>) -> crate::rules::store::MockRuleStore {
    let map: BTreeMap<String, Rule> = rules.into_iter().map(|r| (r.name.clone(), r)).collect();
    let mut store = crate::rules::store::MockRuleStore::new();
    store.expect_load_all().returning(move || Ok(map.clone()));
    store
}

fn url_rule() -> Rule {
    let mut rule = Rule::new("url");
    rule.auto_compatible = true;
    rule.priority = 10;
    rule.regex = Some(r"^https?://\S+$".to_owned());
    rule
}

fn plain_catch_all_rule() -> Rule {
    let mut rule = Rule::new("plain");
    rule.auto_compatible = true;
    rule.priority = 999;
    rule
}

fn wifi_rule() -> Rule {
    let mut rule = Rule::new("wifi");
    rule.auto_compatible = true;
    rule.priority = 5;
    rule.wifi_connect = true;
    rule
}

#[test]
fn route_auto_matches_first_by_priority() {
    let mut low = Rule::new("low");
    low.auto_compatible = true;
    low.priority = 50;
    low.regex = Some(r"^LOW:".to_owned());

    let store = mock_store(vec![url_rule(), low.clone()]);
    let result = route_payload(
        &store,
        RouteMode::Auto(AutoRouteOptions::default()),
        "https://example.com",
    )
    .expect("route");

    assert_eq!(result.payload, "https://example.com");
    assert!(matches!(
        result.event,
        RoutingEvent::Matched {
            rule,
            auto_route: true
        } if rule == "url"
    ));
    let routed = result.routed.expect("should match url");
    assert_eq!(routed.rule.name, "url");
    assert_eq!(routed.vars.raw(), Some("https://example.com"));
}

#[test]
fn route_auto_tries_next_rule_on_regex_no_match() {
    let store = mock_store(vec![url_rule(), plain_catch_all_rule()]);
    let result = route_payload(
        &store,
        RouteMode::Auto(AutoRouteOptions::default()),
        "not-a-url",
    )
    .expect("route");

    assert!(matches!(
        result.event,
        RoutingEvent::Matched {
            rule,
            auto_route: true
        } if rule == "plain"
    ));
    assert_eq!(result.routed.expect("plain catch-all").rule.name, "plain");
}

#[test]
fn route_auto_excludes_reserved_builtin_names() {
    let mut copy_rule = Rule::new("copy");
    copy_rule.auto_compatible = true;
    copy_rule.priority = 1;

    let store = mock_store(vec![copy_rule, plain_catch_all_rule()]);
    let result = route_payload(
        &store,
        RouteMode::Auto(AutoRouteOptions::default()),
        "anything",
    )
    .expect("route");

    assert_eq!(result.routed.expect("plain").rule.name, "plain");
}

#[test]
fn route_auto_excludes_except_names() {
    let store = mock_store(vec![url_rule(), plain_catch_all_rule()]);
    let mut except = HashSet::new();
    except.insert("url".to_owned());
    let options = AutoRouteOptions { except, only: None };

    let result =
        route_payload(&store, RouteMode::Auto(options), "https://example.com").expect("route");

    assert_eq!(result.routed.expect("plain fallback").rule.name, "plain");
}

#[test]
fn route_auto_only_whitelist() {
    let store = mock_store(vec![url_rule(), wifi_rule(), plain_catch_all_rule()]);
    let mut only = HashSet::new();
    only.insert("wifi".to_owned());
    let options = AutoRouteOptions {
        except: HashSet::new(),
        only: Some(only),
    };
    let payload = "WIFI:T:WPA;S:corp;P:secret;;";

    let result = route_payload(&store, RouteMode::Auto(options), payload).expect("route");

    let routed = result.routed.expect("wifi");
    assert_eq!(routed.rule.name, "wifi");
    assert_eq!(routed.vars.get("QR_NATIVE_WIFI_SSID"), Some("corp"));
}

#[test]
fn route_auto_wifi_matches_without_regex() {
    let store = mock_store(vec![wifi_rule(), url_rule(), plain_catch_all_rule()]);
    let payload = "WIFI:T:WPA;S:guest;P:pass;;";

    let result = route_payload(
        &store,
        RouteMode::Auto(AutoRouteOptions::default()),
        payload,
    )
    .expect("route");

    assert_eq!(result.routed.expect("wifi").rule.name, "wifi");
}

#[test]
fn route_auto_no_match_when_no_candidates() {
    let store = mock_store(vec![Rule::new("manual")]);
    let result = route_payload(
        &store,
        RouteMode::Auto(AutoRouteOptions::default()),
        "payload",
    )
    .expect("route");

    assert!(result.routed.is_none());
    assert_eq!(result.event, RoutingEvent::NoAutoMatch);
}

#[test]
fn route_explicit_matches_named_rule() {
    let mut asset = Rule::new("asset");
    asset.regex = Some(r"^ASSET:(?P<asset>\d+)$".to_owned());

    let mut store = crate::rules::store::MockRuleStore::new();
    store
        .expect_get()
        .with(mockall::predicate::eq("asset"))
        .times(1)
        .returning(move |_| Ok(asset.clone()));

    let result =
        route_payload(&store, RouteMode::Explicit("asset".to_owned()), "ASSET:42").expect("route");

    assert!(matches!(
        result.event,
        RoutingEvent::Matched {
            rule,
            auto_route: false
        } if rule == "asset"
    ));
    assert_eq!(
        result.routed.expect("routed").vars.get("QR_VAR_ASSET"),
        Some("42")
    );
}

#[test]
fn route_explicit_mismatch_returns_event() {
    let mut asset = Rule::new("asset");
    asset.regex = Some(r"^ASSET:(?P<asset>\d+)$".to_owned());

    let mut store = crate::rules::store::MockRuleStore::new();
    store
        .expect_get()
        .with(mockall::predicate::eq("asset"))
        .returning(move |_| Ok(asset.clone()));

    let result =
        route_payload(&store, RouteMode::Explicit("asset".to_owned()), "not-asset").expect("route");

    assert!(result.routed.is_none());
    assert_eq!(
        result.event,
        RoutingEvent::Mismatch {
            rule: "asset".to_owned()
        }
    );
}

#[test]
fn route_explicit_not_found_propagates_error() {
    let mut store = crate::rules::store::MockRuleStore::new();
    store
        .expect_get()
        .with(mockall::predicate::eq("missing"))
        .returning(|_| Err(RuleError::NotFound("missing".to_owned())));

    let err = route_payload(&store, RouteMode::Explicit("missing".to_owned()), "x")
        .expect_err("not found");
    assert_eq!(err, RuleError::NotFound("missing".to_owned()));
}

#[test]
fn route_builtin_copy_skips_store() {
    let mut store = crate::rules::store::MockRuleStore::new();
    store.expect_load_all().never();
    store.expect_get().never();

    let result = route_payload(&store, RouteMode::BuiltinCopy, "secret-payload").expect("route");

    assert_eq!(result.payload, "secret-payload");
    assert!(result.routed.is_none());
    assert_eq!(result.event, RoutingEvent::BuiltinCopy);
}

#[test]
fn route_builtin_plain_skips_store() {
    let mut store = crate::rules::store::MockRuleStore::new();
    store.expect_load_all().never();
    store.expect_get().never();

    let result = route_payload(&store, RouteMode::BuiltinPlain, "hello").expect("route");

    assert_eq!(result.payload, "hello");
    assert!(result.routed.is_none());
    assert_eq!(result.event, RoutingEvent::BuiltinPlain);
}

#[test]
fn rule_deserializes_without_new_fields() {
    let json = r#"{"name":"legacy","regex":"^x$"}"#;
    let rule: Rule = serde_json::from_str(json).expect("deserialize");
    assert!(!rule.auto_compatible);
    assert_eq!(rule.priority, 100);
}
