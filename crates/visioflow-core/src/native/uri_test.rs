use std::collections::HashMap;

use super::{NativeParser, UriParser};

fn uri_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn parses_https_uri_with_path() {
    let parser = UriParser;
    let raw = "https://example.com/path/to/page";

    let vars = parser.parse(raw);

    assert_eq!(
        vars,
        uri_map(&[
            ("QR_NATIVE_URI_SCHEME", "https"),
            ("QR_NATIVE_URI_HOST", "example.com"),
            ("QR_NATIVE_URI_PATH", "/path/to/page"),
        ])
    );
    assert!(!vars.contains_key("QR_NATIVE_URI_PORT"));
}

#[test]
fn parses_http_uri_with_explicit_port() {
    let parser = UriParser;
    let raw = "http://localhost:8080/api/v1";

    let vars = parser.parse(raw);

    assert_eq!(
        vars,
        uri_map(&[
            ("QR_NATIVE_URI_SCHEME", "http"),
            ("QR_NATIVE_URI_HOST", "localhost"),
            ("QR_NATIVE_URI_PORT", "8080"),
            ("QR_NATIVE_URI_PATH", "/api/v1"),
        ])
    );
}

#[test]
fn parses_ftp_uri() {
    let parser = UriParser;
    let raw = "ftp://files.server.com/pub/data.bin";

    let vars = parser.parse(raw);

    assert_eq!(
        vars,
        uri_map(&[
            ("QR_NATIVE_URI_SCHEME", "ftp"),
            ("QR_NATIVE_URI_HOST", "files.server.com"),
            ("QR_NATIVE_URI_PATH", "/pub/data.bin"),
        ])
    );
}

#[test]
fn omits_path_key_when_uri_has_no_path_segment() {
    let parser = UriParser;
    let raw = "https://example.com";

    let vars = parser.parse(raw);

    assert_eq!(vars.get("QR_NATIVE_URI_SCHEME").map(String::as_str), Some("https"));
    assert_eq!(vars.get("QR_NATIVE_URI_HOST").map(String::as_str), Some("example.com"));
    assert!(!vars.contains_key("QR_NATIVE_URI_PATH"));
    assert!(!vars.contains_key("QR_NATIVE_URI_PORT"));
}

#[test]
fn returns_empty_for_non_uri_payload() {
    let parser = UriParser;

    assert!(parser.parse("WIFI:T:WPA;S:Home;P:pass;;").is_empty());
    assert!(parser.parse("mailto:user@example.com").is_empty());
    assert!(parser.parse("not-a-uri").is_empty());
}
