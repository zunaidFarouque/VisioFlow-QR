use std::collections::HashMap;

use super::util::percent_decode;
use super::NativeParser;

const SMSTO_PREFIX: &str = "smsto:";
const SMS_PREFIX: &str = "sms:";

/// Parses SMS payloads (`SMSTO:...` or RFC 5724 `sms:...`) into `QR_NATIVE_SMS_*` keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct SmsParser;

impl NativeParser for SmsParser {
    fn parse(&self, raw: &str) -> HashMap<String, String> {
        parse_sms(raw).unwrap_or_default()
    }
}

fn parse_sms(raw: &str) -> Option<HashMap<String, String>> {
    let trimmed = raw.trim();

    if trimmed.len() >= SMSTO_PREFIX.len()
        && trimmed[..SMSTO_PREFIX.len()].eq_ignore_ascii_case(SMSTO_PREFIX)
    {
        let rest = &trimmed[SMSTO_PREFIX.len()..];
        let (number, body) = match rest.split_once(':') {
            Some((num, b)) => (num.trim(), Some(b.trim())),
            None => (rest.trim(), None),
        };

        if number.is_empty() {
            return None;
        }

        let mut out = HashMap::new();
        out.insert("QR_NATIVE_SMS_NUMBER".to_string(), number.to_string());
        if let Some(b) = body {
            if !b.is_empty() {
                out.insert("QR_NATIVE_SMS_BODY".to_string(), b.to_string());
            }
        }
        return Some(out);
    }

    if trimmed.len() >= SMS_PREFIX.len()
        && trimmed[..SMS_PREFIX.len()].eq_ignore_ascii_case(SMS_PREFIX)
    {
        let rest = &trimmed[SMS_PREFIX.len()..];
        let (num_part, query_part) = match rest.split_once('?') {
            Some((num, q)) => (num, Some(q)),
            None => (rest, None),
        };

        let number = percent_decode(num_part.trim());
        if number.is_empty() {
            return None;
        }

        let mut out = HashMap::new();
        out.insert("QR_NATIVE_SMS_NUMBER".to_string(), number);

        if let Some(query) = query_part {
            for pair in query.split('&') {
                let Some((k, v)) = pair.split_once('=') else {
                    continue;
                };
                if k.trim().eq_ignore_ascii_case("body") {
                    let body = percent_decode(v.trim());
                    if !body.is_empty() {
                        out.insert("QR_NATIVE_SMS_BODY".to_string(), body);
                    }
                }
            }
        }

        return Some(out);
    }

    None
}
