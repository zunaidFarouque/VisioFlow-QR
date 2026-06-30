use std::collections::HashMap;

use super::NativeParser;

/// Parses `geo:` payloads into `QR_NATIVE_GEO_*` keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct GeoParser;

impl NativeParser for GeoParser {
    fn parse(&self, raw: &str) -> HashMap<String, String> {
        parse_geo(raw).unwrap_or_default()
    }
}

fn parse_geo(raw: &str) -> Option<HashMap<String, String>> {
    let rest = strip_geo_prefix(raw)?;
    if rest.is_empty() {
        return None;
    }

    let (lat, lon, _alt) = match rest.splitn(3, ',').collect::<Vec<_>>()[..] {
        [lat, lon] => (lat, lon, None),
        [lat, lon, alt] => (lat, lon, Some(alt)),
        _ => return None,
    };

    if lat.is_empty() || lon.is_empty() {
        return None;
    }

    let mut out = HashMap::new();
    out.insert("QR_NATIVE_GEO_LAT".to_string(), lat.to_string());
    out.insert("QR_NATIVE_GEO_LON".to_string(), lon.to_string());
    Some(out)
}

fn strip_geo_prefix(raw: &str) -> Option<&str> {
    raw.strip_prefix("geo:")
        .or_else(|| raw.strip_prefix("GEO:"))
        .or_else(|| {
            if raw.len() >= 4 && raw[..4].eq_ignore_ascii_case("geo:") {
                Some(&raw[4..])
            } else {
                None
            }
        })
}
