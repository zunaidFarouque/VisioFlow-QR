use std::path::{Path, PathBuf};

use crate::encode::internet_shortcut::{is_internet_shortcut_path, parse_internet_shortcut_file};
use crate::error::{Result, VisioFlowError};

/// Resolve clipboard text and optional file-drop paths into a single QR payload string.
pub fn resolve_encode_payload(
    clipboard_text: Option<&str>,
    file_paths: &[PathBuf],
) -> Result<String> {
    if let Some(text) = clipboard_text {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            if let Some(url) = try_resolve_shortcut_reference(trimmed)? {
                return Ok(url);
            }
            return Ok(trimmed.to_string());
        }
    }

    for path in file_paths {
        if is_internet_shortcut_path(path) {
            return parse_internet_shortcut_file(path);
        }
    }

    if !file_paths.is_empty() {
        return Err(VisioFlowError::Encode(
            "no encodable text on clipboard".to_string(),
        ));
    }

    Err(VisioFlowError::Encode("clipboard is empty".to_string()))
}

fn try_resolve_shortcut_reference(text: &str) -> Result<Option<String>> {
    let path_text = strip_file_uri_prefix(text);
    let path = Path::new(path_text);
    if !is_internet_shortcut_path(path) {
        return Ok(None);
    }
    if !path.exists() {
        return Ok(None);
    }
    parse_internet_shortcut_file(path).map(Some)
}

fn strip_file_uri_prefix(text: &str) -> &str {
    text.strip_prefix("file:///")
        .or_else(|| text.strip_prefix("file://"))
        .unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn passes_through_plain_url() {
        let payload = resolve_encode_payload(Some("https://example.com"), &[]).expect("resolve");
        assert_eq!(payload, "https://example.com");
    }

    #[test]
    fn passes_through_plain_text() {
        let payload = resolve_encode_payload(Some("meeting notes"), &[]).expect("resolve");
        assert_eq!(payload, "meeting notes");
    }

    #[test]
    fn extracts_url_from_shortcut_path_on_clipboard() {
        let mut file = NamedTempFile::with_suffix(".url").expect("temp file");
        writeln!(
            file,
            "[InternetShortcut]\nURL=https://inner.example/path\n"
        )
        .expect("write");
        file.flush().expect("flush");
        let path = file.path().to_string_lossy().to_string();
        let payload = resolve_encode_payload(Some(&path), &[]).expect("resolve");
        assert_eq!(payload, "https://inner.example/path");
    }

    #[test]
    fn extracts_url_from_file_uri_shortcut_path() {
        let mut file = NamedTempFile::with_suffix(".url").expect("temp file");
        writeln!(file, "[InternetShortcut]\nURL=https://uri.example\n").expect("write");
        file.flush().expect("flush");
        let path = file.path().to_string_lossy().replace('\\', "/");
        let uri = format!("file:///{path}");
        let payload = resolve_encode_payload(Some(&uri), &[]).expect("resolve");
        assert_eq!(payload, "https://uri.example");
    }

    #[test]
    fn extracts_url_from_dropped_shortcut_file() {
        let mut file = NamedTempFile::with_suffix(".url").expect("temp file");
        writeln!(file, "[InternetShortcut]\nURL=https://drop.example\n").expect("write");
        file.flush().expect("flush");
        let payload =
            resolve_encode_payload(None, &[file.path().to_path_buf()]).expect("resolve");
        assert_eq!(payload, "https://drop.example");
    }

    #[test]
    fn errors_on_empty_clipboard() {
        let err = resolve_encode_payload(None, &[]).expect_err("empty");
        assert!(matches!(err, VisioFlowError::Encode(msg) if msg == "clipboard is empty"));
    }

    #[test]
    fn errors_on_whitespace_only_clipboard() {
        let err = resolve_encode_payload(Some("   \n\t"), &[]).expect_err("blank");
        assert!(matches!(err, VisioFlowError::Encode(msg) if msg == "clipboard is empty"));
    }
}
