use super::{NativeParser, SmsParser};

#[test]
fn parses_smsto_with_body() {
    let payload = "SMSTO:+1234567890:Hello from VisioFlow";
    let vars = SmsParser.parse(payload);

    assert_eq!(vars.get("QR_NATIVE_SMS_NUMBER").map(String::as_str), Some("+1234567890"));
    assert_eq!(vars.get("QR_NATIVE_SMS_BODY").map(String::as_str), Some("Hello from VisioFlow"));
}

#[test]
fn parses_smsto_without_body() {
    let payload = "smsto:999";
    let vars = SmsParser.parse(payload);

    assert_eq!(vars.get("QR_NATIVE_SMS_NUMBER").map(String::as_str), Some("999"));
    assert_eq!(vars.get("QR_NATIVE_SMS_BODY"), None);
}

#[test]
fn parses_smsto_with_colons_in_body() {
    let payload = "SMSTO:+12345:Note: Meeting time: 3pm";
    let vars = SmsParser.parse(payload);

    assert_eq!(vars.get("QR_NATIVE_SMS_NUMBER").map(String::as_str), Some("+12345"));
    assert_eq!(vars.get("QR_NATIVE_SMS_BODY").map(String::as_str), Some("Note: Meeting time: 3pm"));
}

#[test]
fn parses_rfc5724_sms_uri_with_body_query() {
    let payload = "sms:+1987654321?body=Meeting%20at%2010%3A00am";
    let vars = SmsParser.parse(payload);

    assert_eq!(vars.get("QR_NATIVE_SMS_NUMBER").map(String::as_str), Some("+1987654321"));
    assert_eq!(vars.get("QR_NATIVE_SMS_BODY").map(String::as_str), Some("Meeting at 10:00am"));
}

#[test]
fn parses_plain_sms_uri() {
    let payload = "SMS:+1112223333";
    let vars = SmsParser.parse(payload);

    assert_eq!(vars.get("QR_NATIVE_SMS_NUMBER").map(String::as_str), Some("+1112223333"));
    assert_eq!(vars.get("QR_NATIVE_SMS_BODY"), None);
}

#[test]
fn returns_empty_for_non_sms_payload() {
    assert!(SmsParser.parse("mailto:alice@example.com").is_empty());
    assert!(SmsParser.parse("tel:+12345").is_empty());
    assert!(SmsParser.parse("WIFI:S:MyNet;;").is_empty());
}
