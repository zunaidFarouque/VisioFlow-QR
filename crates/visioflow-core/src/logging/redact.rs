use std::collections::HashMap;

/// Placeholder emitted in place of sensitive environment values in logs.
pub const REDACTED: &str = "[REDACTED]";

/// Substrings matched case-insensitively against env-var keys (see `ENGINE_RULES.md` §2).
const SENSITIVE_KEY_PATTERNS: &[&str] = &["PASSWORD", "SECRET", "OTP_SECRET", "TOKEN"];

/// Returns `true` when `key` names a value that must not appear in daemon logs.
pub fn is_sensitive_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    SENSITIVE_KEY_PATTERNS
        .iter()
        .any(|pattern| upper.contains(pattern))
}

/// Returns `[REDACTED]` for sensitive keys; otherwise returns `value` unchanged.
pub fn redact_sensitive(key: &str, value: &str) -> String {
    if is_sensitive_key(key) {
        REDACTED.to_string()
    } else {
        value.to_string()
    }
}

/// Applies [`redact_sensitive`] to every entry in `map`.
pub fn redact_env_map(map: &HashMap<String, String>) -> HashMap<String, String> {
    map.iter()
        .map(|(key, value)| (key.clone(), redact_sensitive(key, value)))
        .collect()
}

/// Formats a single `key=value` log line with sensitive values redacted.
pub fn format_log_line(key: &str, value: &str) -> String {
    format!("{key}={}", redact_sensitive(key, value))
}
