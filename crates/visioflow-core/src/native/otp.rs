use std::collections::HashMap;

use super::util::percent_decode;
use super::NativeParser;

const PREFIX: &str = "otpauth://";

/// Parses Key URI 2FA OTP payloads (`otpauth://totp/...` or `otpauth://hotp/...`)
/// into `QR_NATIVE_OTP_*` keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct OtpParser;

impl NativeParser for OtpParser {
    fn parse(&self, raw: &str) -> HashMap<String, String> {
        parse_otp(raw).unwrap_or_default()
    }
}

fn parse_otp(raw: &str) -> Option<HashMap<String, String>> {
    let raw_trimmed = raw.trim();
    if raw_trimmed.len() < PREFIX.len()
        || !raw_trimmed[..PREFIX.len()].eq_ignore_ascii_case(PREFIX)
    {
        return None;
    }

    let rest = &raw_trimmed[PREFIX.len()..];
    let (type_and_label, query_str) = match rest.split_once('?') {
        Some((tl, q)) => (tl, Some(q)),
        None => (rest, None),
    };

    let (otp_type, label_part) = match type_and_label.split_once('/') {
        Some((t, l)) => (t.to_ascii_lowercase(), l),
        None => return None,
    };

    if otp_type != "totp" && otp_type != "hotp" {
        return None;
    }

    let decoded_label = percent_decode(label_part.trim_start_matches('/'));
    let (label_issuer, label_account) = match decoded_label.split_once(':') {
        Some((iss, acc)) => (Some(iss.trim().to_string()), Some(acc.trim().to_string())),
        None => {
            let acc = decoded_label.trim();
            if acc.is_empty() {
                (None, None)
            } else {
                (None, Some(acc.to_string()))
            }
        }
    };

    let mut secret: Option<String> = None;
    let mut query_issuer: Option<String> = None;
    let mut algorithm: Option<String> = None;
    let mut digits: Option<String> = None;
    let mut period: Option<String> = None;
    let mut counter: Option<String> = None;

    if let Some(query) = query_str {
        for pair in query.split('&') {
            let Some((k, v)) = pair.split_once('=') else {
                continue;
            };
            let key = k.trim().to_ascii_lowercase();
            let val = percent_decode(v.trim());
            if val.is_empty() {
                continue;
            }

            match key.as_str() {
                "secret" => {
                    let cleaned = val.chars().filter(|c| !c.is_whitespace()).collect::<String>();
                    if !cleaned.is_empty() {
                        secret = Some(cleaned);
                    }
                }
                "issuer" => query_issuer = Some(val),
                "algorithm" => algorithm = Some(val.to_ascii_uppercase()),
                "digits" => digits = Some(val),
                "period" => period = Some(val),
                "counter" => counter = Some(val),
                _ => {}
            }
        }
    }

    let secret = secret?;
    let issuer = query_issuer.or(label_issuer);

    let mut out = HashMap::new();
    out.insert("QR_NATIVE_OTP_TYPE".to_string(), otp_type.clone());
    out.insert("QR_NATIVE_OTP_SECRET".to_string(), secret);

    if let Some(iss) = issuer {
        if !iss.is_empty() {
            out.insert("QR_NATIVE_OTP_ISSUER".to_string(), iss);
        }
    }

    if let Some(acc) = label_account {
        if !acc.is_empty() {
            out.insert("QR_NATIVE_OTP_ACCOUNT".to_string(), acc);
        }
    }

    out.insert(
        "QR_NATIVE_OTP_ALGORITHM".to_string(),
        algorithm.unwrap_or_else(|| "SHA1".to_string()),
    );
    out.insert(
        "QR_NATIVE_OTP_DIGITS".to_string(),
        digits.unwrap_or_else(|| "6".to_string()),
    );

    if otp_type == "totp" {
        out.insert(
            "QR_NATIVE_OTP_PERIOD".to_string(),
            period.unwrap_or_else(|| "30".to_string()),
        );
    } else if let Some(cnt) = counter {
        out.insert("QR_NATIVE_OTP_COUNTER".to_string(), cnt);
    }

    Some(out)
}
