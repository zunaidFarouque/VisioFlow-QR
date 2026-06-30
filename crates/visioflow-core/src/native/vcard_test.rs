use std::collections::HashMap;

use super::{NativeParser, VcardParser};

fn vcard_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

const MINIMAL_VCARD: &str = "\
BEGIN:VCARD
VERSION:3.0
FN:Jane Doe
TEL:+1-555-0199
END:VCARD";

#[test]
fn parses_minimal_vcard_fn_and_tel() {
    let parser = VcardParser;

    let vars = parser.parse(MINIMAL_VCARD);

    assert_eq!(
        vars,
        vcard_map(&[
            ("QR_NATIVE_VCARD_FN", "Jane Doe"),
            ("QR_NATIVE_VCARD_TEL", "+1-555-0199"),
        ])
    );
}

#[test]
fn parses_vcard_fn_only_when_no_simple_tel() {
    let parser = VcardParser;
    let raw = "\
BEGIN:VCARD
FN:John Smith
END:VCARD";

    let vars = parser.parse(raw);

    assert_eq!(vars, vcard_map(&[("QR_NATIVE_VCARD_FN", "John Smith")]));
    assert!(!vars.contains_key("QR_NATIVE_VCARD_TEL"));
}

#[test]
fn returns_empty_when_not_vcard() {
    let parser = VcardParser;

    assert!(parser.parse("mailto:user@example.com").is_empty());
    assert!(parser.parse("FN:Ghost").is_empty());
}
