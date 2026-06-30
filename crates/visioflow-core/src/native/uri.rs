use std::collections::HashMap;

use super::NativeParser;

const SUPPORTED_SCHEMES: &[&str] = &["http", "https", "ftp"];

/// Parses `http` / `https` / `ftp` URIs into `QR_NATIVE_URI_*` keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct UriParser;

impl NativeParser for UriParser {
    fn parse(&self, raw: &str) -> HashMap<String, String> {
        parse_uri(raw).unwrap_or_default()
    }
}

fn parse_uri(raw: &str) -> Option<HashMap<String, String>> {
    let (scheme, rest) = raw.split_once("://")?;
    if !SUPPORTED_SCHEMES.contains(&scheme) {
        return None;
    }

    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }

    let slash_idx = rest.find(&['/', '?', '#'][..]);
    let (authority, path_part) = match slash_idx {
        Some(idx) => (&rest[..idx], Some(&rest[idx..])),
        None => (rest, None),
    };

    let host_port = authority.rsplit('@').next()?;
    let (host, port) = split_host_port(host_port)?;

    let mut out = HashMap::new();
    out.insert("QR_NATIVE_URI_SCHEME".to_string(), scheme.to_string());
    out.insert("QR_NATIVE_URI_HOST".to_string(), host);
    if let Some(port) = port {
        out.insert("QR_NATIVE_URI_PORT".to_string(), port);
    }
    if let Some(path) = path_part {
        let path = strip_query_and_fragment(path);
        if !path.is_empty() {
            out.insert("QR_NATIVE_URI_PATH".to_string(), path);
        }
    }

    Some(out)
}

fn strip_query_and_fragment(path: &str) -> String {
    let end = path
        .find('?')
        .or_else(|| path.find('#'))
        .unwrap_or(path.len());
    path[..end].to_string()
}

fn split_host_port(authority: &str) -> Option<(String, Option<String>)> {
    if authority.is_empty() {
        return None;
    }

    if authority.starts_with('[') {
        let end = authority.find(']')?;
        let host = authority[..=end].to_string();
        let remainder = &authority[end + 1..];
        let port = remainder.strip_prefix(':').map(str::to_string);
        return Some((host, port));
    }

    if let Some((host, port)) = authority.rsplit_once(':') {
        if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) && !host.contains(':') {
            return Some((host.to_string(), Some(port.to_string())));
        }
    }

    Some((authority.to_string(), None))
}
