use std::collections::HashMap;

use super::{keys::SENSITIVE_NATIVE_KEYS, NativeParser, WifiParser};

fn wifi_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn parses_standard_wpa_wifi_payload() {
    let parser = WifiParser;
    let raw = "WIFI:T:WPA;S:MyHome;P:secret123;;";

    let vars = parser.parse(raw);

    assert_eq!(
        vars,
        wifi_map(&[
            ("QR_NATIVE_WIFI_ENCRYPTION", "WPA"),
            ("QR_NATIVE_WIFI_SSID", "MyHome"),
            ("QR_NATIVE_WIFI_PASSWORD", "secret123"),
        ])
    );
    assert!(SENSITIVE_NATIVE_KEYS.contains(&"QR_NATIVE_WIFI_PASSWORD"));
}

#[test]
fn parses_nopass_wifi_without_password_field() {
    let parser = WifiParser;
    let raw = "WIFI:T:nopass;S:OpenNet;;";

    let vars = parser.parse(raw);

    assert_eq!(
        vars,
        wifi_map(&[
            ("QR_NATIVE_WIFI_ENCRYPTION", "nopass"),
            ("QR_NATIVE_WIFI_SSID", "OpenNet"),
        ])
    );
    assert!(!vars.contains_key("QR_NATIVE_WIFI_PASSWORD"));
}

#[test]
fn parses_wep_with_hidden_flag() {
    let parser = WifiParser;
    let raw = "WIFI:T:WEP;S:HiddenNet;P:wepkey;H:true;;";

    let vars = parser.parse(raw);

    assert_eq!(
        vars,
        wifi_map(&[
            ("QR_NATIVE_WIFI_ENCRYPTION", "WEP"),
            ("QR_NATIVE_WIFI_SSID", "HiddenNet"),
            ("QR_NATIVE_WIFI_PASSWORD", "wepkey"),
            ("QR_NATIVE_WIFI_HIDDEN", "true"),
        ])
    );
}

#[test]
fn unescapes_special_characters_in_wifi_fields() {
    let parser = WifiParser;
    let raw = r"WIFI:T:WPA;S:My\;Network;P:pa\;ss;H:false;;";

    let vars = parser.parse(raw);

    assert_eq!(vars.get("QR_NATIVE_WIFI_SSID").map(String::as_str), Some("My;Network"));
    assert_eq!(
        vars.get("QR_NATIVE_WIFI_PASSWORD").map(String::as_str),
        Some("pa;ss")
    );
    assert_eq!(vars.get("QR_NATIVE_WIFI_HIDDEN").map(String::as_str), Some("false"));
}

#[test]
fn returns_empty_for_non_wifi_payload() {
    let parser = WifiParser;

    assert!(parser.parse("https://example.com").is_empty());
    assert!(parser.parse("not-wifi").is_empty());
}
