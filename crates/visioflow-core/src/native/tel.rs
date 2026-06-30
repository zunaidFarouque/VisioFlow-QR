use std::collections::HashMap;

use super::NativeParser;

/// Parses `tel:` payloads into `QR_NATIVE_TEL_*` keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct TelParser;

impl NativeParser for TelParser {
    fn parse(&self, raw: &str) -> HashMap<String, String> {
        parse_tel(raw).unwrap_or_default()
    }
}

fn parse_tel(raw: &str) -> Option<HashMap<String, String>> {
    let rest = strip_tel_prefix(raw)?;
    if rest.is_empty() {
        return None;
    }

    let mut out = HashMap::new();
    out.insert("QR_NATIVE_TEL_NUMBER".to_string(), rest.to_string());
    Some(out)
}

fn strip_tel_prefix(raw: &str) -> Option<&str> {
    raw.strip_prefix("tel:")
        .or_else(|| raw.strip_prefix("TEL:"))
        .or_else(|| {
            if raw.len() >= 4 && raw[..4].eq_ignore_ascii_case("tel:") {
                Some(&raw[4..])
            } else {
                None
            }
        })
}
