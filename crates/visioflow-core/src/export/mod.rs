use std::collections::HashMap;

use crate::rules::ResolvedVars;

/// Build export variables from decoded capture payloads (capture-only path).
#[must_use]
pub fn vars_from_payloads(payloads: &[String]) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    if payloads.is_empty() {
        return vars;
    }
    let raw = if payloads.len() == 1 {
        payloads[0].clone()
    } else {
        payloads.join("\n")
    };
    vars.insert(ResolvedVars::QR_RAW.to_string(), raw);
    vars
}

/// Copy resolved rule variables into a map for export emission.
#[must_use]
pub fn vars_from_resolved(resolved: &ResolvedVars) -> HashMap<String, String> {
    resolved
        .iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// Emit bash `export KEY='value'` lines for parent-shell `eval`.
#[must_use]
pub fn emit_bash(vars: &HashMap<String, String>) -> String {
    emit_sorted(vars, |key, value| format!("export {key}={}", bash_quote(value)))
}

/// Emit PowerShell `$env:KEY = 'value'` lines for parent-shell dot-sourcing.
#[must_use]
pub fn emit_ps1(vars: &HashMap<String, String>) -> String {
    emit_sorted(vars, |key, value| format!("$env:{key} = {}", ps1_quote(value)))
}

fn emit_sorted<F>(vars: &HashMap<String, String>, line: F) -> String
where
    F: Fn(&str, &str) -> String,
{
    let mut keys: Vec<&str> = vars.keys().map(String::as_str).collect();
    keys.sort_unstable();

    let mut out = String::new();
    for key in keys {
        if let Some(value) = vars.get(key) {
            out.push_str(&line(key, value));
            out.push('\n');
        }
    }
    out
}

/// Single-quoted string safe for bash `export` values.
fn bash_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Single-quoted string safe for PowerShell (double single-quotes).
fn ps1_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod export_test;
