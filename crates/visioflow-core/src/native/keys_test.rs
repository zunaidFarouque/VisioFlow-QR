use super::is_sensitive_native_key;

#[test]
fn marks_wifi_password_as_sensitive() {
    assert!(is_sensitive_native_key("QR_NATIVE_WIFI_PASSWORD"));
}

#[test]
fn does_not_mark_non_sensitive_keys() {
    assert!(!is_sensitive_native_key("QR_NATIVE_WIFI_SSID"));
    assert!(!is_sensitive_native_key("QR_NATIVE_URI_HOST"));
}
