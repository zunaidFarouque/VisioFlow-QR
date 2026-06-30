use std::collections::HashMap;

use super::NativeParser;

const PREFIX: &str = "mailto:";

/// Parses `mailto:` payloads into `QR_NATIVE_MAIL_*` keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct MailtoParser;

impl NativeParser for MailtoParser {
    fn parse(&self, raw: &str) -> HashMap<String, String> {
        parse_mailto(raw).unwrap_or_default()
    }
}

fn parse_mailto(raw: &str) -> Option<HashMap<String, String>> {
    let rest = raw.strip_prefix(PREFIX)?;
    if rest.is_empty() {
        return None;
    }

    let (address, query) = match rest.split_once('?') {
        Some((addr, q)) => (addr, Some(q)),
        None => (rest, None),
    };

    if address.is_empty() {
        return None;
    }

    let mut out = HashMap::new();
    out.insert("QR_NATIVE_MAIL_TO".to_string(), address.to_string());

    if let Some(query) = query {
        for pair in query.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            if key.eq_ignore_ascii_case("subject") && !value.is_empty() {
                out.insert("QR_NATIVE_MAIL_SUBJECT".to_string(), value.to_string());
            }
        }
    }

    Some(out)
}
