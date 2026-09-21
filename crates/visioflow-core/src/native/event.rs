use std::collections::HashMap;

use super::NativeParser;

const VEVENT_BEGIN: &str = "BEGIN:VEVENT";

/// Parses iCalendar / vEvent payloads (`BEGIN:VEVENT` or `BEGIN:VCALENDAR...`)
/// into `QR_NATIVE_EVENT_*` keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct EventParser;

impl NativeParser for EventParser {
    fn parse(&self, raw: &str) -> HashMap<String, String> {
        parse_event(raw).unwrap_or_default()
    }
}

fn parse_event(raw: &str) -> Option<HashMap<String, String>> {
    if !raw
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case(VEVENT_BEGIN))
    {
        return None;
    }

    let unfolded = unfold_lines(raw);
    let mut out = HashMap::new();

    for line in unfolded {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some((prop_part, value)) = line.split_once(':') else {
            continue;
        };

        let prop_name = prop_part
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_uppercase();

        let unescaped_val = unescape_ical_value(value.trim());
        if unescaped_val.is_empty() {
            continue;
        }

        let key = match prop_name.as_str() {
            "SUMMARY" => "QR_NATIVE_EVENT_SUMMARY",
            "DTSTART" => "QR_NATIVE_EVENT_START",
            "DTEND" => "QR_NATIVE_EVENT_END",
            "LOCATION" => "QR_NATIVE_EVENT_LOCATION",
            "DESCRIPTION" => "QR_NATIVE_EVENT_DESCRIPTION",
            _ => continue,
        };

        out.entry(key.to_string()).or_insert(unescaped_val);
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Unfolds RFC 5545 lines: lines beginning with SPACE or TAB continue the previous line.
fn unfold_lines(raw: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for line in raw.lines() {
        if (line.starts_with(' ') || line.starts_with('\t')) && !lines.is_empty() {
            let last = lines.last_mut().expect("lines is not empty");
            last.push_str(&line[1..]);
        } else {
            lines.push(line.to_string());
        }
    }
    lines
}

/// Unescapes standard RFC 5545 escape sequences: `\,`, `\;`, `\\`, `\n`, `\N`.
fn unescape_ical_value(val: &str) -> String {
    let mut out = String::with_capacity(val.len());
    let mut chars = val.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some(',') => out.push(','),
                Some(';') => out.push(';'),
                Some('\\') => out.push('\\'),
                Some('n') | Some('N') => out.push('\n'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }

    out
}
