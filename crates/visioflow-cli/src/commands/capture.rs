use clap::ValueEnum;
use std::process::Command;
use visioflow_core::capture::CaptureEngine;
use visioflow_core::decode::RqrrDecoder;
use visioflow_core::error::Result;
use visioflow_core::traits::{FrameSource, OpticalFilterKind};
use visioflow_core::{
    FileRuleStore, ResolvedVars, RoutedPayload, Rule, RuleEngine, RuleError, RuleResult,
    RuleStore,
};

use crate::capture::{FileFrameSource, SnipFrameSource};
#[cfg(feature = "opencv-webcam")]
use crate::webcam_session::{capture_webcam_with_preview, WebcamTiming, DEFAULT_WEBCAM_TIMEOUT_SECS};

#[cfg(not(feature = "opencv-webcam"))]
const DEFAULT_WEBCAM_TIMEOUT_SECS: u64 = 20;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CaptureSource {
    Snip,
    Webcam,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum CaptureFilter {
    #[default]
    Otsu,
    Median,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CaptureAction {
    Stdout,
    Copy,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum PreviewPosition {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    #[default]
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum ExposureBracketMode {
    /// Probe at startup and disable bracketing if override plunges the preview dark.
    #[default]
    Auto,
    /// Always run sparse exposure bracket cycling.
    On,
    /// Keep auto exposure only; never override.
    Off,
}

#[derive(Debug, Clone)]
pub struct CaptureArgs {
    pub source: CaptureSource,
    pub filter: CaptureFilter,
    pub action: CaptureAction,
    pub input_image: Option<std::path::PathBuf>,
    pub timeout_secs: u64,
    pub verbose: bool,
    pub preview_position: PreviewPosition,
    pub preview_scale: f32,
    pub exposure_step_ms: u64,
    pub exposure_flush_grabs: u32,
    pub decode_interval_ms: u64,
    pub exposure_bracket: ExposureBracketMode,
    pub trigger: Option<String>,
    pub rule_store: Option<std::path::PathBuf>,
}

impl CaptureArgs {
    pub fn timeout_secs_or_default(timeout_secs: u64) -> u64 {
        if timeout_secs == 0 {
            DEFAULT_WEBCAM_TIMEOUT_SECS
        } else {
            timeout_secs
        }
    }
}

impl From<CaptureFilter> for OpticalFilterKind {
    fn from(value: CaptureFilter) -> Self {
        match value {
            CaptureFilter::Otsu => OpticalFilterKind::Otsu,
            CaptureFilter::Median => OpticalFilterKind::Median,
        }
    }
}

pub fn run_capture(args: CaptureArgs) -> Result<Vec<String>> {
    let filter: OpticalFilterKind = args.filter.into();
    let decoder = RqrrDecoder;

    if let Some(path) = args.input_image {
        let engine = CaptureEngine::new(FileFrameSource::new(path), decoder);
        return engine.run(filter);
    }

    match args.source {
        CaptureSource::Snip => {
            let engine = CaptureEngine::new(SnipFrameSource, decoder);
            engine.run(filter)
        }
        CaptureSource::Webcam => {
            #[cfg(feature = "opencv-webcam")]
            {
                capture_webcam_with_preview(
                    filter,
                    CaptureArgs::timeout_secs_or_default(args.timeout_secs),
                    args.verbose,
                    args.preview_position,
                    args.preview_scale,
                    WebcamTiming::from_ms(
                        args.exposure_step_ms,
                        args.exposure_flush_grabs,
                        args.decode_interval_ms,
                    ),
                    args.exposure_bracket,
                )
            }
            #[cfg(not(feature = "opencv-webcam"))]
            {
                Err(visioflow_core::VisioFlowError::Capture(
                    "webcam capture requires the opencv-webcam feature".into(),
                ))
            }
        }
    }
}

pub fn write_capture_output(payloads: &[String], action: CaptureAction, silent: bool) -> Result<()> {
    match action {
        CaptureAction::Stdout => {
            if !silent {
                for payload in payloads {
                    println!("{payload}");
                }
            }
        }
        CaptureAction::Copy => {
            let combined = payloads.join("\n");
            let mut clipboard = arboard::Clipboard::new().map_err(|e| {
                visioflow_core::VisioFlowError::Capture(format!("clipboard unavailable: {e}"))
            })?;
            clipboard.set_text(combined).map_err(|e| {
                visioflow_core::VisioFlowError::Capture(format!("clipboard write failed: {e}"))
            })?;
            if !silent {
                eprintln!("copied {} payload(s) to clipboard", payloads.len());
            }
        }
    }
    Ok(())
}

/// Test hook: run capture with an injected frame source.
pub fn run_capture_with_source<S: FrameSource>(
    source: S,
    filter: OpticalFilterKind,
) -> Result<Vec<String>> {
    let engine = CaptureEngine::new(source, RqrrDecoder);
    engine.run(filter)
}

/// Route the first captured payload through a named rule (regex + native parsers).
pub fn route_capture_trigger(
    store: &FileRuleStore,
    rule_name: &str,
    payloads: &[String],
) -> RuleResult<RoutedPayload> {
    let payload = payloads.first().ok_or_else(|| {
        RuleError::StoreIo("no payloads decoded for trigger".to_owned())
    })?;
    let engine = RuleEngine::new(store.clone());
    engine.route_fully(rule_name, payload).map_err(|err| {
        map_trigger_error(err, payloads, rule_name, store)
    })
}

/// Enrich regex mismatch errors with the decoded payload and rule pattern.
pub fn map_trigger_error(
    err: RuleError,
    payloads: &[String],
    rule_name: &str,
    store: &FileRuleStore,
) -> RuleError {
    if err != RuleError::NoMatch {
        return err;
    }

    let decoded = payloads
        .first()
        .map(|p| format!("{p:?}"))
        .unwrap_or_else(|| "<empty>".to_owned());
    let pattern = store
        .get(rule_name)
        .ok()
        .and_then(|rule| rule.regex)
        .map(|pattern| format!("; rule '{rule_name}' expects pattern: {pattern}"))
        .unwrap_or_default();

    RuleError::StoreIo(format!(
        "regex did not match decoded payload {decoded}{pattern}"
    ))
}

/// Spawn the rule's exec action with resolved variables in the child environment.
pub fn spawn_rule_exec(rule: &Rule, vars: &ResolvedVars) -> RuleResult<()> {
    let Some(exec) = rule.exec.as_ref() else {
        return Ok(());
    };

    let mut cmd = Command::new(exec);
    for (key, value) in vars.iter() {
        cmd.env(key, value);
    }

    cmd.status()
        .map_err(|e| RuleError::ExecFailed(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod trigger_tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;
    use visioflow_core::{apply_rule, merge_native_vars, resolve_payload_fully, Rule, RuleStore};

    fn temp_store() -> (TempDir, FileRuleStore) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("rules.json");
        (dir, FileRuleStore::new(path))
    }

    #[test]
    fn route_capture_trigger_uses_first_payload() {
        let (_dir, store) = temp_store();
        let mut rule = Rule::new("asset");
        rule.regex = Some(r"ASSET:(?P<asset>\d+)".to_owned());
        store.upsert(&rule).expect("upsert");

        let routed = route_capture_trigger(&store, "asset", &["ASSET:7".to_owned(), "skip".to_owned()])
            .expect("route");

        assert_eq!(routed.vars.get("QR_VAR_ASSET"), Some("7"));
    }

    #[test]
    fn route_capture_trigger_rejects_empty_payloads() {
        let (_dir, store) = temp_store();
        let err = route_capture_trigger(&store, "any", &[]).expect_err("empty");
        assert!(matches!(err, RuleError::StoreIo(_)));
    }

    #[test]
    fn map_trigger_error_includes_decoded_payload_and_pattern() {
        let (_dir, store) = temp_store();
        let mut rule = Rule::new("asset");
        rule.regex = Some(r"ASSET:(?P<asset>\d+)".to_owned());
        store.upsert(&rule).expect("upsert");

        let err = route_capture_trigger(&store, "asset", &["https://example.com".to_owned()])
            .expect_err("mismatch");

        let message = err.to_string();
        assert!(message.contains("https://example.com"));
        assert!(message.contains(r"ASSET:(?P<asset>\d+)"));
    }

    #[test]
    fn resolve_payload_fully_integration_wifi() {
        let rule = Rule::new("wifi");
        let payload = "WIFI:T:WPA;S:lab;P:secret;;";
        let resolved = resolve_payload_fully(&rule, payload).expect("resolve");
        assert_eq!(resolved.get("QR_NATIVE_WIFI_SSID"), Some("lab"));
    }

    #[test]
    fn merge_native_vars_does_not_remove_qr_raw() {
        let mut resolved = apply_rule(&Rule::new("plain"), "https://a.test").expect("apply");
        merge_native_vars(&mut resolved, "https://a.test");
        assert_eq!(resolved.raw(), Some("https://a.test"));
        assert_eq!(resolved.get("QR_NATIVE_URI_HOST"), Some("a.test"));
    }

    #[test]
    fn spawn_rule_exec_passes_env_to_child() {
        let dir = TempDir::new().expect("tempdir");
        let out_path = dir.path().join("child-out.txt");
        let script_path = write_env_echo_script(&dir, &out_path);

        let mut rule = Rule::new("run");
        rule.exec = Some(script_path);

        let mut vars = ResolvedVars::new();
        vars.insert("QR_VAR_ASSET", "triggered-99");

        spawn_rule_exec(&rule, &vars).expect("spawn");

        let contents = fs::read_to_string(&out_path).expect("read child output");
        assert!(contents.contains("triggered-99"));
    }

    #[cfg(windows)]
    fn write_env_echo_script(dir: &TempDir, out_path: &Path) -> std::path::PathBuf {
        let script_path = dir.path().join("echo-asset.cmd");
        let body = format!(
            "@echo off\r\necho %QR_VAR_ASSET% > \"{}\"\r\n",
            out_path.display()
        );
        fs::write(&script_path, body).expect("write cmd");
        script_path
    }

    #[cfg(not(windows))]
    fn write_env_echo_script(dir: &TempDir, out_path: &Path) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let script_path = dir.path().join("echo-asset.sh");
        let body = format!(
            "#!/bin/sh\necho \"$QR_VAR_ASSET\" > \"{}\"\n",
            out_path.display()
        );
        fs::write(&script_path, &body).expect("write sh");
        let mut perms = fs::metadata(&script_path).expect("meta").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).expect("chmod");
        script_path
    }
}
