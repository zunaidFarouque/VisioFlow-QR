use super::{NativeParser, OtpParser};

#[test]
fn parses_standard_totp_payload() {
    let payload = "otpauth://totp/Example:alice@google.com?secret=JBSWY3DPEHPK3PXP&issuer=Example";
    let vars = OtpParser.parse(payload);

    assert_eq!(vars.get("QR_NATIVE_OTP_TYPE").map(String::as_str), Some("totp"));
    assert_eq!(vars.get("QR_NATIVE_OTP_SECRET").map(String::as_str), Some("JBSWY3DPEHPK3PXP"));
    assert_eq!(vars.get("QR_NATIVE_OTP_ISSUER").map(String::as_str), Some("Example"));
    assert_eq!(vars.get("QR_NATIVE_OTP_ACCOUNT").map(String::as_str), Some("alice@google.com"));
    assert_eq!(vars.get("QR_NATIVE_OTP_DIGITS").map(String::as_str), Some("6"));
    assert_eq!(vars.get("QR_NATIVE_OTP_PERIOD").map(String::as_str), Some("30"));
    assert_eq!(vars.get("QR_NATIVE_OTP_ALGORITHM").map(String::as_str), Some("SHA1"));
}

#[test]
fn parses_totp_with_url_encoded_label_and_query_overrides() {
    let payload = "otpauth://totp/GitHub%3Aoctocat?secret=HXDMVJECJJWSRB3HWIZR4IFUGFTMXBOZ&issuer=GitHubInc&algorithm=SHA256&digits=8&period=60";
    let vars = OtpParser.parse(payload);

    assert_eq!(vars.get("QR_NATIVE_OTP_TYPE").map(String::as_str), Some("totp"));
    assert_eq!(vars.get("QR_NATIVE_OTP_SECRET").map(String::as_str), Some("HXDMVJECJJWSRB3HWIZR4IFUGFTMXBOZ"));
    // Explicit query issuer overrides prefix in label
    assert_eq!(vars.get("QR_NATIVE_OTP_ISSUER").map(String::as_str), Some("GitHubInc"));
    assert_eq!(vars.get("QR_NATIVE_OTP_ACCOUNT").map(String::as_str), Some("octocat"));
    assert_eq!(vars.get("QR_NATIVE_OTP_ALGORITHM").map(String::as_str), Some("SHA256"));
    assert_eq!(vars.get("QR_NATIVE_OTP_DIGITS").map(String::as_str), Some("8"));
    assert_eq!(vars.get("QR_NATIVE_OTP_PERIOD").map(String::as_str), Some("60"));
}

#[test]
fn parses_hotp_with_counter() {
    let payload = "otpauth://hotp/BankService:user123?secret=KVKFKRCPNZQUYMLX&counter=42";
    let vars = OtpParser.parse(payload);

    assert_eq!(vars.get("QR_NATIVE_OTP_TYPE").map(String::as_str), Some("hotp"));
    assert_eq!(vars.get("QR_NATIVE_OTP_SECRET").map(String::as_str), Some("KVKFKRCPNZQUYMLX"));
    assert_eq!(vars.get("QR_NATIVE_OTP_COUNTER").map(String::as_str), Some("42"));
    assert_eq!(vars.get("QR_NATIVE_OTP_ISSUER").map(String::as_str), Some("BankService"));
    assert_eq!(vars.get("QR_NATIVE_OTP_ACCOUNT").map(String::as_str), Some("user123"));
}

#[test]
fn returns_empty_for_missing_secret() {
    let payload = "otpauth://totp/Example:user";
    let vars = OtpParser.parse(payload);
    assert!(vars.is_empty());
}

#[test]
fn returns_empty_for_non_otp_payload() {
    assert!(OtpParser.parse("https://example.com").is_empty());
    assert!(OtpParser.parse("WIFI:S:MyNet;;").is_empty());
}
