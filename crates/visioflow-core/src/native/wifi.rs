use std::collections::HashMap;

use super::NativeParser;

const PREFIX: &str = "WIFI:";

/// Parses WiFi QR payloads (`WIFI:T:...;S:...;P:...;;`) into `QR_NATIVE_WIFI_*` keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct WifiParser;

impl NativeParser for WifiParser {
    fn parse(&self, raw: &str) -> HashMap<String, String> {
        let Some(body) = raw.strip_prefix(PREFIX) else {
            return HashMap::new();
        };

        let fields = split_wifi_fields(body);
        let mut out = HashMap::new();
        for (key, value) in fields {
            let env_key = match key.as_str() {
                "T" => "QR_NATIVE_WIFI_ENCRYPTION",
                "S" => "QR_NATIVE_WIFI_SSID",
                "P" => "QR_NATIVE_WIFI_PASSWORD",
                "H" => "QR_NATIVE_WIFI_HIDDEN",
                _ => continue,
            };
            out.insert(env_key.to_string(), unescape_wifi_value(&value));
        }
        out
    }
}

fn split_wifi_fields(body: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let mut current_key = String::new();
    let mut current_value = String::new();
    let mut in_value = false;
    let mut escaped = false;

    for ch in body.chars() {
        if escaped {
            if in_value {
                current_value.push(ch);
            } else {
                current_key.push(ch);
            }
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if !in_value {
            if ch == ':' {
                in_value = true;
                continue;
            }
            current_key.push(ch);
            continue;
        }

        if ch == ';' {
            if !current_key.is_empty() {
                fields.push((current_key.clone(), current_value.clone()));
            }
            current_key.clear();
            current_value.clear();
            in_value = false;
            continue;
        }

        current_value.push(ch);
    }

    if !current_key.is_empty() {
        fields.push((current_key, current_value));
    }

    fields
}

fn unescape_wifi_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut escaped = false;

    for ch in value.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        out.push(ch);
    }

    out
}
