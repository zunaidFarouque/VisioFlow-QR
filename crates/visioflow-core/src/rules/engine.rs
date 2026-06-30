use regex::Regex;

use crate::native::{
    GeoParser, MailtoParser, NativeParser, TelParser, UriParser, VcardParser, WifiParser,
};
use crate::rules::error::{Result, RuleError};
use crate::rules::model::{ResolvedVars, Rule};
use crate::rules::store::RuleStore;

/// Routes a payload through a named rule and produces resolved env vars.
#[cfg_attr(test, mockall::automock)]
pub trait PayloadRouter: Send + Sync {
    fn route(&self, rule_name: &str, payload: &str) -> Result<ResolvedVars>;
}

/// Applies a rule to a payload without loading from persistence.
pub fn apply_rule(rule: &Rule, payload: &str) -> Result<ResolvedVars> {
    let mut resolved = ResolvedVars::new();
    resolved.insert(ResolvedVars::QR_RAW, payload);

    let Some(pattern) = rule.regex.as_deref() else {
        return Ok(resolved);
    };

    let regex = Regex::new(pattern).map_err(|e| RuleError::InvalidRegex(e.to_string()))?;
    let captures = regex.captures(payload).ok_or(RuleError::NoMatch)?;

    for name in regex.capture_names().flatten() {
        let Some(value) = captures.name(name).map(|m| m.as_str().to_owned()) else {
            continue;
        };
        let suffix = rule
            .captures
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_ascii_uppercase());
        let var_name = format!("{}{}", ResolvedVars::QR_VAR_PREFIX, suffix);
        resolved.insert(var_name, value);
    }

    Ok(resolved)
}

/// Rule engine backed by a [`RuleStore`].
pub struct RuleEngine<S: RuleStore> {
    store: S,
}

impl<S: RuleStore> RuleEngine<S> {
    #[must_use]
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

impl<S: RuleStore> PayloadRouter for RuleEngine<S> {
    fn route(&self, rule_name: &str, payload: &str) -> Result<ResolvedVars> {
        let rule = self.store.get(rule_name)?;
        apply_rule(&rule, payload)
    }
}

/// Rule plus fully resolved variables (regex + native parsers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedPayload {
    pub rule: Rule,
    pub vars: ResolvedVars,
}

/// Merge built-in `QR_NATIVE_*` variables from protocol parsers into `resolved`.
pub fn merge_native_vars(resolved: &mut ResolvedVars, payload: &str) {
    const PARSERS: &[&dyn NativeParser] = &[
        &WifiParser,
        &UriParser,
        &MailtoParser,
        &TelParser,
        &GeoParser,
        &VcardParser,
    ];
    for parser in PARSERS {
        for (key, value) in parser.parse(payload) {
            resolved.insert(key, value);
        }
    }
}

/// Apply a rule and merge native parser variables for the payload.
pub fn resolve_payload_fully(rule: &Rule, payload: &str) -> Result<ResolvedVars> {
    let mut resolved = apply_rule(rule, payload)?;
    merge_native_vars(&mut resolved, payload);
    Ok(resolved)
}

impl<S: RuleStore> RuleEngine<S> {
    /// Route a payload through a named rule and merge native parser variables.
    pub fn route_fully(&self, rule_name: &str, payload: &str) -> Result<RoutedPayload> {
        let rule = self.store.get(rule_name)?;
        let vars = resolve_payload_fully(&rule, payload)?;
        Ok(RoutedPayload { rule, vars })
    }
}
