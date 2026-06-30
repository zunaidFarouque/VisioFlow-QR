/// Env keys whose values must be redacted before logging (see `ENGINE_RULES.md`).
pub const SENSITIVE_NATIVE_KEYS: &[&str] = &["QR_NATIVE_WIFI_PASSWORD"];

#[must_use]
pub fn is_sensitive_native_key(key: &str) -> bool {
    SENSITIVE_NATIVE_KEYS.contains(&key)
}
