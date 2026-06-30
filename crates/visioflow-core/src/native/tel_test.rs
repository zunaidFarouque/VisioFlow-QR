use std::collections::HashMap;

use super::{NativeParser, TelParser};

fn tel_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn parses_tel_number() {
    let parser = TelParser;
    let raw = "tel:+15551234";

    let vars = parser.parse(raw);

    assert_eq!(vars, tel_map(&[("QR_NATIVE_TEL_NUMBER", "+15551234")]));
}

#[test]
fn parses_tel_case_insensitive_prefix() {
    let parser = TelParser;

    let vars = parser.parse("TEL:8005551212");

    assert_eq!(vars.get("QR_NATIVE_TEL_NUMBER").map(String::as_str), Some("8005551212"));
}

#[test]
fn returns_empty_for_non_tel_payload() {
    let parser = TelParser;

    assert!(parser.parse("mailto:user@example.com").is_empty());
    assert!(parser.parse("geo:48.85,2.35").is_empty());
}
