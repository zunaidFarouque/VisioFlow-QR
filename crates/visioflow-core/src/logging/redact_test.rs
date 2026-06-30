use std::collections::HashMap;

use super::{format_log_line, is_sensitive_key, redact_env_map, redact_sensitive, REDACTED};

const SECRET_PASSWORD: &str = "super-secret-wifi-pass-12345";
const SECRET_TOKEN: &str = "eyJhbGciOiJIUzI1NiJ9.payload.signature";

#[test]
fn redact_sensitive_wifi_password_key() {
    let out = redact_sensitive("QR_NATIVE_WIFI_PASSWORD", SECRET_PASSWORD);
    assert_eq!(out, REDACTED);
    assert!(!out.contains(SECRET_PASSWORD));
}

#[test]
fn redact_sensitive_case_insensitive_key() {
    let out = redact_sensitive("qr_native_wifi_password", SECRET_PASSWORD);
    assert_eq!(out, REDACTED);
    assert!(!out.contains(SECRET_PASSWORD));
}

#[test]
fn redact_sensitive_password_substring_in_key() {
    let out = redact_sensitive("MY_APP_PASSWORD", SECRET_PASSWORD);
    assert_eq!(out, REDACTED);
}

#[test]
fn redact_sensitive_secret_substring_in_key() {
    let out = redact_sensitive("OTP_SECRET", "TOTP_BASE32_SECRET");
    assert_eq!(out, REDACTED);
    assert!(!out.contains("TOTP_BASE32_SECRET"));
}

#[test]
fn redact_sensitive_token_substring_in_key() {
    let out = redact_sensitive("ACCESS_TOKEN", SECRET_TOKEN);
    assert_eq!(out, REDACTED);
    assert!(!out.contains(SECRET_TOKEN));
}

#[test]
fn redact_sensitive_preserves_safe_keys() {
    assert_eq!(
        redact_sensitive("QR_RAW", "WIFI:T:MyNet;P:ignored-in-raw;;"),
        "WIFI:T:MyNet;P:ignored-in-raw;;"
    );
    assert_eq!(redact_sensitive("QR_NATIVE_WIFI_SSID", "MyNet"), "MyNet");
    assert_eq!(redact_sensitive("QR_VAR_ASSET", "42"), "42");
}

#[test]
fn is_sensitive_key_detects_all_patterns() {
    assert!(is_sensitive_key("QR_NATIVE_WIFI_PASSWORD"));
    assert!(is_sensitive_key("OTP_SECRET"));
    assert!(is_sensitive_key("API_TOKEN"));
    assert!(is_sensitive_key("CLIENT_SECRET"));
    assert!(!is_sensitive_key("QR_RAW"));
    assert!(!is_sensitive_key("QR_NATIVE_WIFI_SSID"));
}

#[test]
fn redact_env_map_redacts_sensitive_entries_only() {
    let mut map = HashMap::new();
    map.insert(
        "QR_NATIVE_WIFI_PASSWORD".to_string(),
        SECRET_PASSWORD.to_string(),
    );
    map.insert("QR_NATIVE_WIFI_SSID".to_string(), "MyNet".to_string());
    map.insert("QR_RAW".to_string(), "payload".to_string());
    map.insert("ACCESS_TOKEN".to_string(), SECRET_TOKEN.to_string());

    let redacted = redact_env_map(&map);

    assert_eq!(redacted.get("QR_NATIVE_WIFI_PASSWORD").unwrap(), REDACTED);
    assert_eq!(redacted.get("QR_NATIVE_WIFI_SSID").unwrap(), "MyNet");
    assert_eq!(redacted.get("QR_RAW").unwrap(), "payload");
    assert_eq!(redacted.get("ACCESS_TOKEN").unwrap(), REDACTED);

    let serialized = format!("{redacted:?}");
    assert!(!serialized.contains(SECRET_PASSWORD));
    assert!(!serialized.contains(SECRET_TOKEN));
}

#[test]
fn format_log_line_never_leaks_sensitive_values() {
    let line = format_log_line("QR_NATIVE_WIFI_PASSWORD", SECRET_PASSWORD);
    assert_eq!(line, format!("QR_NATIVE_WIFI_PASSWORD={REDACTED}"));
    assert!(!line.contains(SECRET_PASSWORD));

    let safe = format_log_line("QR_NATIVE_WIFI_SSID", "MyNet");
    assert_eq!(safe, "QR_NATIVE_WIFI_SSID=MyNet");
}
