/// Reserved rule/trigger names that cannot be created via `rule create`.
pub const RESERVED_RULE_NAMES: &[&str] = &["copy", "plain", "auto"];

/// Returns true when `name` is a reserved builtin identifier.
#[must_use]
pub fn is_reserved_rule_name(name: &str) -> bool {
    RESERVED_RULE_NAMES.contains(&name)
}

/// Returns true when `name` is a builtin trigger (`copy` or `plain`), not a store rule.
#[must_use]
pub fn is_builtin_trigger(name: &str) -> bool {
    matches!(name, "copy" | "plain")
}

/// Reserved names that must never appear in the auto-routing candidate pool.
#[must_use]
pub fn is_excluded_from_auto_scan(name: &str) -> bool {
    matches!(name, "copy" | "auto")
}
