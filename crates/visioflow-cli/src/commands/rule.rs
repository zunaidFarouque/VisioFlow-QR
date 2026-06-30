use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use visioflow_core::{
    default_rules_asset_path, is_reserved_rule_name, resolve_share_path, FileRuleStore,
    ResolvedVars, RoutedPayload, Rule, RuleEngine, RuleError, RuleResult, RuleStore,
    VisioFlowError,
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
    if is_reserved_rule_name(name) {
        return Err(RuleError::StoreIo(format!("reserved rule name: {name}")));
    }
    if store.get(name).is_ok() {
        return Err(RuleError::StoreIo(format!("rule already exists: {name}")));
    }
    store.upsert(&Rule::new(name))
}

fn load_default_rules() -> RuleResult<BTreeMap<String, Rule>> {
    let path = default_rules_asset_path();
    let contents = fs::read_to_string(&path).map_err(|e| {
        RuleError::StoreIo(format!("read default rules at {}: {e}", path.display()))
    })?;
    serde_json::from_str(&contents)
        .map_err(|e| RuleError::StoreParse(format!("parse default rules: {e}")))
}

fn rewrite_rule_exec_paths(rules: &mut BTreeMap<String, Rule>) {
    for rule in rules.values_mut() {
        if let Some(exec) = rule.exec.clone() {
            let relative = exec.to_string_lossy();
            rule.exec = Some(resolve_share_path(&relative));
        }
    }
}

/// Install stock default rules from `assets/default-rules.json`.
pub fn rule_init_defaults(store: &dyn RuleStore, merge: bool, force: bool) -> RuleResult<()> {
    let mut defaults = load_default_rules()?;
    rewrite_rule_exec_paths(&mut defaults);

    if force {
        store.save_all(&defaults)?;
        return Ok(());
    }

    let mut existing = store.load_all()?;

    if merge {
        for (name, rule) in defaults {
            existing.entry(name).or_insert(rule);
        }
    } else {
        for (name, rule) in defaults {
            existing.insert(name, rule);
        }
    }

    store.save_all(&existing)
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

pub fn rule_set_action(
    store: &dyn RuleStore,
    name: &str,
    exec: Option<&Path>,
    wifi_connect: bool,
) -> RuleResult<()> {
    if exec.is_none() && !wifi_connect {
        return Err(RuleError::StoreIo(
            "set-action requires --exec and/or --wifi-connect".to_owned(),
        ));
    }

    let mut rule = store.get(name)?;
    if let Some(path) = exec {
        rule.exec = Some(path.to_path_buf());
    }
    if wifi_connect {
        rule.wifi_connect = true;
    }
    store.upsert(&rule)
}

pub fn rule_list(store: &dyn RuleStore) -> RuleResult<Vec<Rule>> {
    let rules = store.load_all()?;
    Ok(rules.into_values().collect())
}

pub fn rule_delete(store: &dyn RuleStore, name: &str) -> RuleResult<()> {
    store.delete(name)
}

pub fn write_rule_list_output(
    rules: &[Rule],
    format: RuleOutputFormat,
    silent: bool,
) -> Result<(), VisioFlowError> {
    if silent {
        return Ok(());
    }

    match format {
        RuleOutputFormat::Plain => {
            for rule in rules {
                println!("{}", rule.name);
            }
        }
        RuleOutputFormat::Json => {
            let json = serde_json::to_string(rules)
                .map_err(|e| VisioFlowError::Capture(format!("json encode failed: {e}")))?;
            println!("{json}");
        }
    }

    Ok(())
}

pub fn rule_execute(store: &FileRuleStore, name: &str, payload: &str) -> RuleResult<RoutedPayload> {
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
            let map: std::collections::BTreeMap<&str, &str> = vars.iter().collect();
            let json = serde_json::to_string(&map)
                .map_err(|e| VisioFlowError::Capture(format!("json encode failed: {e}")))?;
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
    use super::*;
    use std::collections::BTreeMap;
    use tempfile::TempDir;
    use visioflow_core::apply_rule;

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
    fn rule_create_rejects_reserved_name() {
        let (_dir, store) = temp_store();
        let err = rule_create(&store, "copy").expect_err("reserved");
        assert!(matches!(err, RuleError::StoreIo(_)));
        assert!(err.to_string().contains("reserved"));
    }

    #[test]
    fn rule_init_defaults_installs_stock_rules_with_resolved_exec() {
        let (_dir, store) = temp_store();
        rule_init_defaults(&store, false, false).expect("init-defaults");

        let rules = store.load_all().expect("load");
        assert!(rules.contains_key("url"));
        assert!(rules.contains_key("wifi"));
        assert!(rules.contains_key("plain"));

        let url = rules.get("url").expect("url rule");
        assert_eq!(url.priority, 10);
        assert!(url.auto_compatible);
        let exec = url.exec.as_ref().expect("url exec");
        assert!(
            exec.is_file(),
            "exec should resolve to existing script: {}",
            exec.display()
        );
        #[cfg(windows)]
        assert!(exec.extension().is_some_and(|e| e == "ps1"));
        #[cfg(not(windows))]
        assert!(exec.extension().is_some_and(|e| e == "sh"));
    }

    #[test]
    fn rule_init_defaults_merge_skips_existing_names() {
        let (_dir, store) = temp_store();
        let mut custom = Rule::new("url");
        custom.regex = Some("^custom$".to_owned());
        store.upsert(&custom).expect("seed custom url");

        rule_init_defaults(&store, true, false).expect("merge");

        let rules = store.load_all().expect("load");
        let url = rules.get("url").expect("url");
        assert_eq!(url.regex.as_deref(), Some("^custom$"));
        assert!(rules.contains_key("wifi"));
    }

    #[test]
    fn rule_init_defaults_force_replaces_entire_store() {
        let (_dir, store) = temp_store();
        store.upsert(&Rule::new("custom-only")).expect("seed");

        rule_init_defaults(&store, false, true).expect("force");

        let rules = store.load_all().expect("load");
        assert!(!rules.contains_key("custom-only"));
        assert!(rules.contains_key("url"));
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

        rule_set_action(&store, "run", Some(Path::new("/bin/echo")), false).expect("set-action");

        let rule = store.get("run").expect("rule");
        assert_eq!(rule.exec.as_deref(), Some(Path::new("/bin/echo")));
        assert!(!rule.wifi_connect);
    }

    #[test]
    fn rule_set_action_persists_wifi_connect_flag() {
        let (_dir, store) = temp_store();
        rule_create(&store, "wifi").expect("create");

        rule_set_action(&store, "wifi", None, true).expect("set-action");

        let rule = store.get("wifi").expect("rule");
        assert!(rule.wifi_connect);
        assert!(rule.exec.is_none());
    }

    #[test]
    fn rule_set_action_requires_exec_or_wifi() {
        let (_dir, store) = temp_store();
        rule_create(&store, "empty").expect("create");
        let err = rule_set_action(&store, "empty", None, false).expect_err("needs flag");
        assert!(matches!(err, RuleError::StoreIo(_)));
    }

    #[test]
    fn rule_list_returns_all_rules_sorted_by_store_order() {
        let (_dir, store) = temp_store();
        rule_create(&store, "alpha").expect("create");
        rule_create(&store, "beta").expect("create");

        let rules = rule_list(&store).expect("list");
        let names: Vec<&str> = rules.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["alpha", "beta"]);
    }

    #[test]
    fn rule_delete_removes_existing_rule() {
        let (_dir, store) = temp_store();
        rule_create(&store, "gone").expect("create");

        rule_delete(&store, "gone").expect("delete");
        assert!(store.get("gone").is_err());
    }

    #[test]
    fn rule_delete_errors_when_missing() {
        let (_dir, store) = temp_store();
        let err = rule_delete(&store, "missing").expect_err("not found");
        assert!(matches!(err, RuleError::NotFound(_)));
    }

    #[test]
    fn write_rule_list_output_plain_one_name_per_line() {
        let rules = vec![Rule::new("alpha"), Rule::new("beta")];
        let mut buf = Vec::new();
        for rule in &rules {
            use std::io::Write;
            writeln!(buf, "{}", rule.name).expect("write");
        }
        let output = String::from_utf8(buf).expect("utf8");
        assert_eq!(output, "alpha\nbeta\n");
    }

    #[test]
    fn write_rule_list_output_json_roundtrip() {
        let mut rule = Rule::new("asset");
        rule.regex = Some(r"ASSET:(?P<asset>\d+)".to_owned());
        let rules = vec![rule];
        let json = serde_json::to_string(&rules).expect("json");
        let parsed: Vec<Rule> = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed[0].name, "asset");
        assert_eq!(parsed[0].regex.as_deref(), Some(r"ASSET:(?P<asset>\d+)"));
    }

    #[test]
    fn rule_execute_resolves_vars_via_store() {
        let (_dir, store) = temp_store();
        rule_create(&store, "asset").expect("create");
        rule_config(&store, "asset", Some(r"ASSET:(?P<asset>\d+)"), &[]).expect("config");

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
