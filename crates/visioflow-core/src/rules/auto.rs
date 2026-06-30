use std::collections::HashSet;

use crate::native::{NativeParser, WifiParser};
use crate::rules::builtins::is_excluded_from_auto_scan;
use crate::rules::engine::{resolve_payload_fully, RoutedPayload};
use crate::rules::error::{Result, RuleError};
use crate::rules::model::Rule;
use crate::rules::store::RuleStore;

/// Options that constrain auto-routing candidate selection.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AutoRouteOptions {
    /// Rule names excluded from the auto scan.
    pub except: HashSet<String>,
    /// When set, only these rule names are considered (stricter than `except`).
    pub only: Option<HashSet<String>>,
}

/// How a payload should be routed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteMode {
    Auto(AutoRouteOptions),
    Explicit(String),
    BuiltinCopy,
    BuiltinPlain,
}

/// Routing outcome notification for stderr / JSON output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingEvent {
    Matched { rule: String, auto_route: bool },
    Mismatch { rule: String },
    NoAutoMatch,
    BuiltinCopy,
    BuiltinPlain,
}

/// Result of routing a decoded payload (resolution only; actions are CLI-owned).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteResult {
    pub routed: Option<RoutedPayload>,
    pub event: RoutingEvent,
    pub payload: String,
}

/// Route `payload` according to `mode` using rules from `store`.
pub fn route_payload<S: RuleStore>(
    store: &S,
    mode: RouteMode,
    payload: &str,
) -> Result<RouteResult> {
    let payload_owned = payload.to_owned();
    match mode {
        RouteMode::BuiltinCopy => Ok(RouteResult {
            routed: None,
            event: RoutingEvent::BuiltinCopy,
            payload: payload_owned,
        }),
        RouteMode::BuiltinPlain => Ok(RouteResult {
            routed: None,
            event: RoutingEvent::BuiltinPlain,
            payload: payload_owned,
        }),
        RouteMode::Explicit(rule_name) => route_explicit(store, &rule_name, payload),
        RouteMode::Auto(options) => route_auto(store, &options, payload),
    }
}

fn route_explicit<S: RuleStore>(store: &S, rule_name: &str, payload: &str) -> Result<RouteResult> {
    let rule = store.get(rule_name)?;
    match try_match_rule(&rule, payload, rule.priority) {
        Ok(Some(routed)) => Ok(RouteResult {
            routed: Some(routed),
            event: RoutingEvent::Matched {
                rule: rule_name.to_owned(),
                auto_route: false,
            },
            payload: payload.to_owned(),
        }),
        Ok(None) => Ok(RouteResult {
            routed: None,
            event: RoutingEvent::Mismatch {
                rule: rule_name.to_owned(),
            },
            payload: payload.to_owned(),
        }),
        Err(e) => Err(e),
    }
}

fn route_auto<S: RuleStore>(
    store: &S,
    options: &AutoRouteOptions,
    payload: &str,
) -> Result<RouteResult> {
    let candidates = auto_candidates(store, options)?;
    if candidates.is_empty() {
        return Ok(RouteResult {
            routed: None,
            event: RoutingEvent::NoAutoMatch,
            payload: payload.to_owned(),
        });
    }

    let catch_all_priority = candidates.iter().map(|r| r.priority).max().unwrap_or(0);

    for rule in &candidates {
        if let Some(routed) = try_match_rule(rule, payload, catch_all_priority)? {
            return Ok(RouteResult {
                routed: Some(routed),
                event: RoutingEvent::Matched {
                    rule: rule.name.clone(),
                    auto_route: true,
                },
                payload: payload.to_owned(),
            });
        }
    }

    Ok(RouteResult {
        routed: None,
        event: RoutingEvent::NoAutoMatch,
        payload: payload.to_owned(),
    })
}

fn auto_candidates<S: RuleStore>(
    store: &S,
    options: &AutoRouteOptions,
) -> Result<Vec<Rule>> {
    let all = store.load_all()?;
    let mut candidates: Vec<Rule> = all
        .into_values()
        .filter(|rule| rule.auto_compatible)
        .filter(|rule| !is_excluded_from_auto_scan(&rule.name))
        .filter(|rule| !options.except.contains(&rule.name))
        .filter(|rule| {
            options
                .only
                .as_ref()
                .is_none_or(|only| only.contains(&rule.name))
        })
        .collect();

    candidates.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.name.cmp(&b.name)));
    Ok(candidates)
}

fn try_match_rule(
    rule: &Rule,
    payload: &str,
    catch_all_priority: u32,
) -> Result<Option<RoutedPayload>> {
    if rule.regex.is_some() {
        return match resolve_payload_fully(rule, payload) {
            Ok(vars) => Ok(Some(RoutedPayload {
                rule: rule.clone(),
                vars,
            })),
            Err(RuleError::NoMatch) => Ok(None),
            Err(e) => Err(e),
        };
    }

    if rule.wifi_connect && wifi_payload_matches(payload) {
        let vars = resolve_payload_fully(rule, payload)?;
        return Ok(Some(RoutedPayload {
            rule: rule.clone(),
            vars,
        }));
    }

    if rule.priority == catch_all_priority {
        let vars = resolve_payload_fully(rule, payload)?;
        return Ok(Some(RoutedPayload {
            rule: rule.clone(),
            vars,
        }));
    }

    Ok(None)
}

fn wifi_payload_matches(payload: &str) -> bool {
    if payload.starts_with("WIFI:") {
        return true;
    }
    WifiParser
        .parse(payload)
        .contains_key("QR_NATIVE_WIFI_SSID")
}
