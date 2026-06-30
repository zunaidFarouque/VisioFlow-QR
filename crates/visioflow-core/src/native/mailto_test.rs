use std::collections::HashMap;

use super::{MailtoParser, NativeParser};

fn mailto_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn parses_mailto_with_subject() {
    let parser = MailtoParser;
    let raw = "mailto:user@example.com?subject=Hi";

    let vars = parser.parse(raw);

    assert_eq!(
        vars,
        mailto_map(&[
            ("QR_NATIVE_MAIL_TO", "user@example.com"),
            ("QR_NATIVE_MAIL_SUBJECT", "Hi"),
        ])
    );
}

#[test]
fn parses_mailto_address_only() {
    let parser = MailtoParser;
    let raw = "mailto:admin@corp.test";

    let vars = parser.parse(raw);

    assert_eq!(
        vars,
        mailto_map(&[("QR_NATIVE_MAIL_TO", "admin@corp.test")])
    );
    assert!(!vars.contains_key("QR_NATIVE_MAIL_SUBJECT"));
}

#[test]
fn returns_empty_for_non_mailto_payload() {
    let parser = MailtoParser;

    assert!(parser.parse("tel:+15551234").is_empty());
    assert!(parser.parse("https://example.com").is_empty());
}
