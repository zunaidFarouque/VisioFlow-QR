use std::collections::HashMap;

use super::NativeParser;

const VCARD_BEGIN: &str = "BEGIN:VCARD";

/// Parses minimal vCard payloads into `QR_NATIVE_VCARD_*` keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct VcardParser;

impl NativeParser for VcardParser {
    fn parse(&self, raw: &str) -> HashMap<String, String> {
        parse_vcard(raw).unwrap_or_default()
    }
}

fn parse_vcard(raw: &str) -> Option<HashMap<String, String>> {
    if !raw
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case(VCARD_BEGIN))
    {
        return None;
    }

    let mut out = HashMap::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some((prop, value)) = line.split_once(':') else {
            continue;
        };

        let prop_upper = prop.to_ascii_uppercase();
        if prop_upper == "FN" || prop_upper.starts_with("FN;") {
            if !value.is_empty() {
                out.insert("QR_NATIVE_VCARD_FN".to_string(), value.to_string());
            }
            continue;
        }

        if is_simple_tel_property(prop) && !value.is_empty() {
            out.entry("QR_NATIVE_VCARD_TEL".to_string())
                .or_insert_with(|| value.to_string());
        }
    }

    if out.is_empty() {
        return None;
    }

    Some(out)
}

fn is_simple_tel_property(prop: &str) -> bool {
    prop.eq_ignore_ascii_case("TEL")
}
