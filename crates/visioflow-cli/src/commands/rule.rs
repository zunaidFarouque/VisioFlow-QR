use std::path::{Path, PathBuf};

use visioflow_core::{
    FileRuleStore, ResolvedVars, RoutedPayload, Rule, RuleEngine, RuleError, RuleResult,
    RuleStore, VisioFlowError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleOutputFormat {
    Plain,
    Json,
}

/// Parse `GROUP:VAR` capture mapping (e.g. `asset:ASSET`).
pub fn parse_capture_map(spec: &str) -> Result<(String, String), String> {
    let (group, var) = spec
        .split_once(':')
        .ok_or_else(|| "map must be GROUP:VAR".to_owned())?;
    if group.is_empty() || var.is_empty() {
        return Err("map must be GROUP:VAR".to_owned());
    }
    Ok((group.to_owned(), var.to_owned()))
}

#[must_use]
pub fn open_store(path: Option<PathBuf>) -> FileRuleStore {
    match path {
        Some(p) => FileRuleStore::new(p),
        None => FileRuleStore::with_default_path(),
    }
}

pub fn rule_create(store: &dyn RuleStore, name: &str) -> RuleResult<()> {
    if store.get(name).is_ok() {
        return Err(RuleError::StoreIo(format!("rule already exists: {name}")));
    }
    store.upsert(&Rule::new(name))
}

pub fn rule_config(
    store: &dyn RuleStore,
    name: &str,
    regex: Option<&str>,
    maps: &[String],
) -> RuleResult<()> {
    if regex.is_none() && maps.is_empty() {
        return Err(RuleError::StoreIo(
            "config requires --regex and/or --map".to_owned(),
        ));
    }

    let mut rule = store.get(name)?;

    if let Some(pattern) = regex {
        rule.regex = Some(pattern.to_owned());
    }

    for spec in maps {
        let (group, var) = parse_capture_map(spec).map_err(RuleError::StoreIo)?;
        rule.captures.insert(group, var);
    }

    store.upsert(&rule)
}

pub fn rule_set_action(store: &dyn RuleStore, name: &str, exec: &Path) -> RuleResult<()> {
    let mut rule = store.get(name)?;
    rule.exec = Some(exec.to_path_buf());
    store.upsert(&rule)
}

pub fn rule_execute(
    store: &FileRuleStore,
    name: &str,
    payload: &str,
) -> RuleResult<RoutedPayload> {
    let engine = RuleEngine::new(store.clone());
    engine.route_fully(name, payload)
}

pub fn write_resolved_output(
    vars: &ResolvedVars,
    format: RuleOutputFormat,
    silent: bool,
) -> Result<(), VisioFlowError> {
    if silent {
        return Ok(());
    }

    match format {
        RuleOutputFormat::Plain => {
            for (key, value) in vars.iter() {
                println!("{key}={value}");
            }
        }
        RuleOutputFormat::Json => {
            let map: std::collections::BTreeMap<&str, &str> =
                vars.iter().collect();
            let json = serde_json::to_string(&map).map_err(|e| {
                VisioFlowError::Capture(format!("json encode failed: {e}"))
            })?;
            println!("{json}");
        }
    }

    Ok(())
}

pub fn map_rule_error(err: RuleError) -> VisioFlowError {
    VisioFlowError::Capture(err.to_string())
}

#[cfg(test)]
mod tests {
    use visioflow_core::apply_rule;
    use super::*;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    fn temp_store() -> (TempDir, FileRuleStore) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("rules.json");
        (dir, FileRuleStore::new(path))
    }

    #[test]
    fn parse_capture_map_accepts_group_var() {
        assert_eq!(
            parse_capture_map("asset:ASSET").expect("valid"),
            ("asset".to_owned(), "ASSET".to_owned())
        );
    }

    #[test]
    fn parse_capture_map_rejects_missing_colon() {
        assert!(parse_capture_map("assetASSET").is_err());
    }

    #[test]
    fn parse_capture_map_rejects_empty_parts() {
        assert!(parse_capture_map(":VAR").is_err());
        assert!(parse_capture_map("group:").is_err());
    }

    #[test]
    fn rule_create_persists_empty_rule() {
        let (_dir, store) = temp_store();
        rule_create(&store, "demo").expect("create should succeed");

        let rule = store.get("demo").expect("rule should exist");
        assert_eq!(rule.name, "demo");
        assert!(rule.regex.is_none());
        assert!(rule.exec.is_none());
    }

    #[test]
    fn rule_create_rejects_duplicate() {
        let (_dir, store) = temp_store();
        rule_create(&store, "demo").expect("first create");
        let err = rule_create(&store, "demo").expect_err("duplicate");
        assert!(matches!(err, RuleError::StoreIo(_)));
    }

    #[test]
    fn rule_config_sets_regex_and_maps() {
        let (_dir, store) = temp_store();
        rule_create(&store, "asset").expect("create");

        rule_config(
            &store,
            "asset",
            Some(r"ASSET:(?P<asset>\d+)"),
            &["asset:ASSET".to_owned()],
        )
        .expect("config");

        let rule = store.get("asset").expect("rule");
        assert_eq!(rule.regex.as_deref(), Some(r"ASSET:(?P<asset>\d+)"));
        assert_eq!(rule.captures.get("asset"), Some(&"ASSET".to_owned()));
    }

    #[test]
    fn rule_set_action_persists_exec_path() {
        let (_dir, store) = temp_store();
        rule_create(&store, "run").expect("create");

        rule_set_action(&store, "run", Path::new("/bin/echo"))
            .expect("set-action");

        let rule = store.get("run").expect("rule");
        assert_eq!(rule.exec.as_deref(), Some(Path::new("/bin/echo")));
    }

    #[test]
    fn rule_execute_resolves_vars_via_store() {
        let (_dir, store) = temp_store();
        rule_create(&store, "asset").expect("create");
        rule_config(
            &store,
            "asset",
            Some(r"ASSET:(?P<asset>\d+)"),
            &[],
        )
        .expect("config");

        let resolved = rule_execute(&store, "asset", "ASSET:42").expect("execute");
        assert_eq!(resolved.vars.raw(), Some("ASSET:42"));
        assert_eq!(resolved.vars.get("QR_VAR_ASSET"), Some("42"));
    }

    #[test]
    fn rule_execute_merges_native_wifi_vars() {
        let (_dir, store) = temp_store();
        rule_create(&store, "wifi").expect("create");

        let resolved =
            rule_execute(&store, "wifi", "WIFI:T:WPA;S:lab;P:secret;;").expect("execute");
        assert_eq!(resolved.vars.get("QR_NATIVE_WIFI_SSID"), Some("lab"));
    }

    #[test]
    fn write_resolved_output_plain() {
        let mut vars = ResolvedVars::new();
        vars.insert("QR_RAW", "hello");
        vars.insert("QR_VAR_ASSET", "42");

        let mut buf = Vec::new();
        for (key, value) in vars.iter() {
            use std::io::Write;
            writeln!(buf, "{key}={value}").expect("write");
        }
        let output = String::from_utf8(buf).expect("utf8");
        assert!(output.contains("QR_RAW=hello"));
        assert!(output.contains("QR_VAR_ASSET=42"));
    }

    #[test]
    fn write_resolved_output_json_roundtrip() {
        let mut vars = ResolvedVars::new();
        vars.insert("QR_RAW", "ASSET:42");
        vars.insert("QR_VAR_ASSET", "42");

        let map: BTreeMap<&str, &str> = vars.iter().collect();
        let json = serde_json::to_string(&map).expect("json");
        let parsed: BTreeMap<String, String> = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed.get("QR_RAW"), Some(&"ASSET:42".to_owned()));
        assert_eq!(parsed.get("QR_VAR_ASSET"), Some(&"42".to_owned()));
    }

    #[test]
    fn apply_rule_integration_matches_configured_regex() {
        let mut rule = Rule::new("inline");
        rule.regex = Some(r"id:(?P<id>\w+)".to_owned());
        let resolved = apply_rule(&rule, "id:abc").expect("match");
        assert_eq!(resolved.get("QR_VAR_ID"), Some("abc"));
    }
}
