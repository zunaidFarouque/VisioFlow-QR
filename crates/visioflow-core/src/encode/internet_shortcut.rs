use std::path::Path;

use crate::error::{Result, VisioFlowError};

/// Parse a Windows `.url` or freedesktop `.desktop` link file and return the embedded URL.
pub fn parse_internet_shortcut_file(path: &Path) -> Result<String> {
    let contents = std::fs::read_to_string(path).map_err(|e| {
        VisioFlowError::Encode(format!(
            "failed to read shortcut file {}: {e}",
            path.display()
        ))
    })?;
    parse_internet_shortcut_contents(&contents).ok_or_else(|| {
        VisioFlowError::Encode(format!(
            "no URL= entry found in shortcut file {}",
            path.display()
        ))
    })
}

/// Parse shortcut file contents and return the first `URL=` value.
#[must_use]
pub fn parse_internet_shortcut_contents(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("URL") {
            let url = value.trim();
            if !url.is_empty() {
                return Some(url.to_string());
            }
        }
    }
    None
}

/// Returns true when `path` looks like a shortcut file we can parse for a URL.
#[must_use]
pub fn is_internet_shortcut_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("url") || ext.eq_ignore_ascii_case("desktop"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parses_url_from_internet_shortcut_contents() {
        let contents = "[InternetShortcut]\r\nURL=https://example.com/page\r\nIDList=...\r\n";
        assert_eq!(
            parse_internet_shortcut_contents(contents),
            Some("https://example.com/page".to_string())
        );
    }

    #[test]
    fn parses_url_case_insensitive_key() {
        let contents = "[InternetShortcut]\nurl=https://lower.example\n";
        assert_eq!(
            parse_internet_shortcut_contents(contents),
            Some("https://lower.example".to_string())
        );
    }

    #[test]
    fn parses_url_from_desktop_entry() {
        let contents = "[Desktop Entry]\nType=Link\nURL=https://desktop.example\n";
        assert_eq!(
            parse_internet_shortcut_contents(contents),
            Some("https://desktop.example".to_string())
        );
    }

    #[test]
    fn returns_none_when_url_missing() {
        let contents = "[InternetShortcut]\nIconFile=C:\\icon.ico\n";
        assert_eq!(parse_internet_shortcut_contents(contents), None);
    }

    #[test]
    fn reads_url_from_temp_file() {
        let mut file = NamedTempFile::with_suffix(".url").expect("temp file");
        writeln!(
            file,
            "[InternetShortcut]\nURL=https://file.example/test\n"
        )
        .expect("write");
        file.flush().expect("flush");
        let url = parse_internet_shortcut_file(file.path()).expect("parse");
        assert_eq!(url, "https://file.example/test");
    }

    #[test]
    fn detects_shortcut_extensions() {
        assert!(is_internet_shortcut_path(Path::new("link.url")));
        assert!(is_internet_shortcut_path(Path::new("link.URL")));
        assert!(is_internet_shortcut_path(Path::new("link.desktop")));
        assert!(!is_internet_shortcut_path(Path::new("readme.txt")));
    }
}
