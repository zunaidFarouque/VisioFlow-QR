use clap::ValueEnum;
use std::collections::HashSet;
use std::result::Result as StdResult;
use visioflow_core::capture::CaptureEngine;
use visioflow_core::decode::RqrrDecoder;
use visioflow_core::error::Result;
use visioflow_core::traits::{FrameSource, OpticalFilterKind};
use visioflow_core::{
    FileRuleStore, RouteMode, RoutedPayload, RoutingEvent, RuleEngine, RuleError, RuleResult,
    RuleStore,
};

use crate::capture::{FileFrameSource, SnipFrameSource};
use crate::notifications::{
    send_native_notification, truncate_for_toast, NativeNotification, TOAST_BODY_MAX_CHARS,
};
#[cfg(feature = "opencv-webcam")]
use crate::webcam_session::{
    capture_webcam_with_preview, WebcamTiming, DEFAULT_WEBCAM_TIMEOUT_SECS,
};

pub use crate::commands::exec::spawn_rule_actions;

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

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum OnMismatch {
    #[default]
    Copy,
    None,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum WifiHandoffMode {
    #[default]
    OpenSettings,
    Print,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default, PartialEq, Eq)]
pub enum CaptureNotify {
    Off,
    On,
    #[default]
    ErrorsOnly,
}

#[derive(Debug, Clone)]
pub struct CaptureArgs {
    pub source: CaptureSource,
    pub filter: CaptureFilter,
    pub action: Option<CaptureAction>,
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
    pub except: Vec<String>,
    pub only: Vec<String>,
    pub on_mismatch: OnMismatch,
    pub wifi_handoff: WifiHandoffMode,
    pub rule_store: Option<std::path::PathBuf>,
    pub select: bool,
    pub interactive: bool,
    pub notify: bool,
    /// When true (default), flip webcam frames horizontally for selfie-style preview and decode.
    pub mirror: bool,
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
                    args.mirror,
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

pub fn write_capture_output(
    payloads: &[String],
    action: CaptureAction,
    silent: bool,
) -> Result<()> {
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

/// When `--select` is set and multiple payloads were decoded, prompt on stdin for one choice.
pub fn select_payload_if_needed<R: std::io::BufRead, W: std::io::Write>(
    payloads: &[String],
    select: bool,
    stdin: &mut R,
    prompt: &mut W,
) -> Result<Vec<String>> {
    if !select || payloads.len() <= 1 {
        return Ok(payloads.to_vec());
    }

    writeln!(prompt, "Multiple payloads detected. Select one:")?;
    for (index, payload) in payloads.iter().enumerate() {
        writeln!(prompt, "  [{}] {}", index + 1, payload)?;
    }
    write!(prompt, "Enter number (1-{}): ", payloads.len())?;
    prompt.flush()?;

    let mut line = String::new();
    stdin
        .read_line(&mut line)
        .map_err(visioflow_core::VisioFlowError::Io)?;

    let choice = line.trim();
    let index = choice
        .parse::<usize>()
        .ok()
        .and_then(|n| n.checked_sub(1))
        .filter(|&i| i < payloads.len())
        .ok_or_else(|| {
            visioflow_core::VisioFlowError::Capture(format!(
                "invalid selection '{choice}'; expected 1-{}",
                payloads.len()
            ))
        })?;

    Ok(vec![payloads[index].clone()])
}

/// When `--interactive` is set, print payload(s) and require `[y/N]` confirmation on stdin.
pub fn confirm_capture_interactive<R: std::io::BufRead, W: std::io::Write>(
    payloads: &[String],
    interactive: bool,
    stdin: &mut R,
    prompt: &mut W,
) -> Result<bool> {
    if !interactive {
        return Ok(true);
    }

    if payloads.is_empty() {
        return Ok(false);
    }

    writeln!(prompt, "Decoded payload:")?;
    for payload in payloads {
        writeln!(prompt, "  {payload}")?;
    }
    write!(prompt, "Proceed? [y/N]: ")?;
    prompt.flush()?;

    let mut line = String::new();
    stdin
        .read_line(&mut line)
        .map_err(visioflow_core::VisioFlowError::Io)?;

    Ok(matches!(line.trim(), "y" | "Y" | "yes" | "Yes" | "YES"))
}

/// Apply `--select` then `--interactive` halts before action/trigger/export.
pub fn apply_capture_halts<R: std::io::BufRead, W: std::io::Write>(
    payloads: Vec<String>,
    select: bool,
    interactive: bool,
    stdin: &mut R,
    prompt: &mut W,
) -> Result<Vec<String>> {
    let selected = select_payload_if_needed(&payloads, select, stdin, prompt)?;
    if !confirm_capture_interactive(&selected, interactive, stdin, prompt)? {
        return Err(visioflow_core::VisioFlowError::Capture(
            "capture cancelled by user".into(),
        ));
    }
    Ok(selected)
}

#[cfg(test)]
mod halt_tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn select_payload_skips_prompt_for_single_payload() {
        let payloads = vec!["only-one".to_owned()];
        let mut stdin = Cursor::new(Vec::new());
        let mut prompt = Vec::new();

        let out =
            select_payload_if_needed(&payloads, true, &mut stdin, &mut prompt).expect("select");

        assert_eq!(out, payloads);
        assert!(prompt.is_empty());
    }

    #[test]
    fn select_payload_skips_prompt_when_disabled() {
        let payloads = vec!["a".to_owned(), "b".to_owned()];
        let mut stdin = Cursor::new(Vec::new());
        let mut prompt = Vec::new();

        let out =
            select_payload_if_needed(&payloads, false, &mut stdin, &mut prompt).expect("select");

        assert_eq!(out, payloads);
        assert!(prompt.is_empty());
    }

    #[test]
    fn select_payload_picks_numbered_choice() {
        let payloads = vec!["first".to_owned(), "second".to_owned(), "third".to_owned()];
        let mut stdin = Cursor::new(b"2\n");
        let mut prompt = Vec::new();

        let out =
            select_payload_if_needed(&payloads, true, &mut stdin, &mut prompt).expect("select");

        assert_eq!(out, vec!["second".to_owned()]);
        let menu = String::from_utf8(prompt).expect("utf8");
        assert!(menu.contains("[1] first"));
        assert!(menu.contains("[2] second"));
        assert!(menu.contains("Enter number (1-3)"));
    }

    #[test]
    fn select_payload_rejects_invalid_choice() {
        let payloads = vec!["a".to_owned(), "b".to_owned()];
        let mut stdin = Cursor::new(b"9\n");
        let mut prompt = Vec::new();

        let err = select_payload_if_needed(&payloads, true, &mut stdin, &mut prompt)
            .expect_err("invalid");

        assert!(err.to_string().contains("invalid selection"));
    }

    #[test]
    fn confirm_interactive_disabled_proceeds() {
        let payloads = vec!["x".to_owned()];
        let mut stdin = Cursor::new(Vec::new());
        let mut prompt = Vec::new();

        let proceed = confirm_capture_interactive(&payloads, false, &mut stdin, &mut prompt)
            .expect("confirm");

        assert!(proceed);
        assert!(prompt.is_empty());
    }

    #[test]
    fn confirm_interactive_accepts_y() {
        let payloads = vec!["payload-a".to_owned()];
        let mut stdin = Cursor::new(b"y\n");
        let mut prompt = Vec::new();

        let proceed =
            confirm_capture_interactive(&payloads, true, &mut stdin, &mut prompt).expect("confirm");

        assert!(proceed);
        let text = String::from_utf8(prompt).expect("utf8");
        assert!(text.contains("payload-a"));
        assert!(text.contains("[y/N]"));
    }

    #[test]
    fn confirm_interactive_defaults_no_on_empty_input() {
        let payloads = vec!["payload-b".to_owned()];
        let mut stdin = Cursor::new(b"\n");
        let mut prompt = Vec::new();

        let proceed =
            confirm_capture_interactive(&payloads, true, &mut stdin, &mut prompt).expect("confirm");

        assert!(!proceed);
    }

    #[test]
    fn apply_capture_halts_select_then_confirm() {
        let payloads = vec!["one".to_owned(), "two".to_owned()];
        let mut stdin = Cursor::new(b"2\nyes\n");
        let mut prompt = Vec::new();

        let out =
            apply_capture_halts(payloads, true, true, &mut stdin, &mut prompt).expect("halts");

        assert_eq!(out, vec!["two".to_owned()]);
    }

    #[test]
    fn apply_capture_halts_cancelled_on_interactive_no() {
        let payloads = vec!["only".to_owned()];
        let mut stdin = Cursor::new(b"n\n");
        let mut prompt = Vec::new();

        let err = apply_capture_halts(payloads, false, true, &mut stdin, &mut prompt)
            .expect_err("cancelled");

        assert!(err.to_string().contains("cancelled"));
    }
}

/// Test hook: run capture with an injected frame source.
pub fn run_capture_with_source<S: FrameSource>(
    source: S,
    filter: OpticalFilterKind,
) -> Result<Vec<String>> {
    let engine = CaptureEngine::new(source, RqrrDecoder);
    engine.run(filter)
}

/// Outcome of applying routing after capture halts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingApplyResult {
    Matched(RoutedPayload),
    CopiedPayload { event: RoutingEvent },
    PrintedPayload(String),
}

/// Build routing mode from CLI `--trigger`, `--except`, and `--only`.
#[must_use]
pub fn route_mode_from_trigger(
    trigger: Option<&str>,
    except: &[String],
    only: &[String],
) -> RouteMode {
    let except_set: HashSet<String> = except.iter().cloned().collect();
    let only_set = (!only.is_empty()).then(|| only.iter().cloned().collect());
    match trigger {
        None => RouteMode::Auto(visioflow_core::AutoRouteOptions {
            except: except_set.clone(),
            only: only_set.clone(),
        }),
        Some("copy") => RouteMode::BuiltinCopy,
        Some("plain") => RouteMode::BuiltinPlain,
        Some("auto") => RouteMode::Auto(visioflow_core::AutoRouteOptions {
            except: except_set,
            only: only_set,
        }),
        Some(name) => RouteMode::Explicit(name.to_owned()),
    }
}

fn format_capture_routing_message(event: &RoutingEvent, fallback_copied: bool) -> String {
    if fallback_copied {
        return visioflow_core::format_routing_message(event);
    }

    match event {
        RoutingEvent::Mismatch { rule } => {
            format!(r#"visioflow: rule "{rule}" did not match"#)
        }
        RoutingEvent::NoAutoMatch => "visioflow: no auto rule matched".to_owned(),
        _ => visioflow_core::format_routing_message(event),
    }
}

fn is_catchall_copy_rule(rule: &visioflow_core::Rule) -> bool {
    rule.regex.is_none() && !rule.wifi_connect && rule.exec.is_none()
}

/// Apply routing after `--select` / `--interactive` halts.
pub fn apply_routing_after_halts(
    store: &FileRuleStore,
    payloads: &[String],
    mode: RouteMode,
    on_mismatch: OnMismatch,
    wifi_handoff: WifiHandoffMode,
    notify: CaptureNotify,
    verbose: bool,
    silent: bool,
) -> Result<RoutingApplyResult> {
    let payload = payloads.first().ok_or_else(|| {
        visioflow_core::VisioFlowError::Capture("no payloads decoded for routing".into())
    })?;

    match &mode {
        RouteMode::BuiltinCopy => {
            if !silent {
                eprintln!(
                    "{}",
                    format_capture_routing_message(&RoutingEvent::BuiltinCopy, false)
                );
            }
            emit_routing_notification(
                notify,
                &RoutingEvent::BuiltinCopy,
                payload,
                verbose,
                silent,
                true,
            );
            write_capture_output(std::slice::from_ref(payload), CaptureAction::Copy, true)?;
            return Ok(RoutingApplyResult::CopiedPayload {
                event: RoutingEvent::BuiltinCopy,
            });
        }
        RouteMode::BuiltinPlain => {
            write_capture_output(std::slice::from_ref(payload), CaptureAction::Stdout, silent)?;
            return Ok(RoutingApplyResult::PrintedPayload(payload.clone()));
        }
        _ => {}
    }

    let route_result =
        visioflow_core::route_payload(store, mode.clone(), payload).map_err(map_routing_error)?;
    if let Some(mut routed) = route_result.routed {
        if routed.vars.get("QR_NATIVE_WIFI_SSID").is_some() {
            let mode_value = match wifi_handoff {
                WifiHandoffMode::OpenSettings => "open-settings",
                WifiHandoffMode::Print => "print",
            };
            routed
                .vars
                .insert("VISIOFLOW_WIFI_HANDOFF_MODE", mode_value);
        }
        if routed.rule.wifi_connect && !silent {
            eprintln!(
                r#"visioflow: connecting to WiFi (rule "{}")"#,
                routed.rule.name
            );
        }

        let event = route_result.event.clone();
        if is_catchall_copy_rule(&routed.rule) {
            if !silent {
                eprintln!(
                    "{}",
                    format_capture_routing_message(&event, false)
                );
            }
            emit_routing_notification(notify, &event, payload, verbose, silent, true);
            write_capture_output(std::slice::from_ref(payload), CaptureAction::Copy, true)?;
            return Ok(RoutingApplyResult::CopiedPayload { event });
        }

        if let Err(err) = notify_then_action(
            notify,
            &event,
            payload,
            verbose,
            silent,
            send_native_notification,
            || spawn_rule_actions(&routed.rule, &routed.vars),
        ) {
            if let Some(note) = wifi_error_notification(&err, &routed.rule.name, payload) {
                emit_native_notification(notify, note, verbose, silent);
            }
            return Err(map_routing_error(err));
        }

        if !silent {
            eprintln!(
                "{}",
                format_capture_routing_message(&event, false)
            );
        }
        return Ok(RoutingApplyResult::Matched(routed));
    }

    let event = route_result.event;
    match on_mismatch {
        OnMismatch::Copy => {
            if !silent {
                eprintln!("{}", format_capture_routing_message(&event, true));
            }
            emit_routing_notification(notify, &event, payload, verbose, silent, true);
            write_capture_output(std::slice::from_ref(payload), CaptureAction::Copy, true)?;
            Ok(RoutingApplyResult::CopiedPayload { event })
        }
        OnMismatch::None => {
            if !silent {
                eprintln!("{}", format_capture_routing_message(&event, false));
            }
            emit_routing_notification(notify, &event, payload, verbose, silent, false);
            Err(visioflow_core::VisioFlowError::Capture(
                "routing failed with --on-mismatch none".into(),
            ))
        }
    }
}

/// Show a routing toast when enabled, then run `action` (rule exec, WiFi, copy, etc.).
pub fn notify_then_action<F, S>(
    mode: CaptureNotify,
    event: &RoutingEvent,
    payload: &str,
    verbose: bool,
    silent: bool,
    sender: S,
    action: F,
) -> std::result::Result<(), RuleError>
where
    S: Fn(&NativeNotification) -> StdResult<(), String>,
    F: FnOnce() -> std::result::Result<(), RuleError>,
{
    emit_routing_notification_with(mode, event, payload, verbose, silent, false, sender);
    action()
}

/// Public entry for capture/IPC paths that need notify-before-action ordering.
pub fn notify_routing_outcome(
    mode: CaptureNotify,
    event: &RoutingEvent,
    payload: &str,
    verbose: bool,
    silent: bool,
) {
    emit_routing_notification(mode, event, payload, verbose, silent, false);
}

fn emit_routing_notification(
    mode: CaptureNotify,
    event: &RoutingEvent,
    payload: &str,
    verbose: bool,
    silent: bool,
    already_copied: bool,
) {
    emit_routing_notification_with(
        mode,
        event,
        payload,
        verbose,
        silent,
        already_copied,
        send_native_notification,
    );
}

fn emit_routing_notification_with<S>(
    mode: CaptureNotify,
    event: &RoutingEvent,
    payload: &str,
    verbose: bool,
    silent: bool,
    already_copied: bool,
    sender: S,
) where
    S: Fn(&NativeNotification) -> StdResult<(), String>,
{
    if !should_notify_for_event(mode, event) {
        return;
    }
    let note = build_routing_notification(event, payload, already_copied);
    emit_native_notification_with(note, verbose, silent, sender);
}

fn emit_native_notification(
    mode: CaptureNotify,
    note: NativeNotification,
    verbose: bool,
    silent: bool,
) {
    if matches!(mode, CaptureNotify::Off) {
        return;
    }
    emit_native_notification_with(note, verbose, silent, send_native_notification);
}

fn emit_native_notification_with<F>(
    note: NativeNotification,
    verbose: bool,
    silent: bool,
    sender: F,
) where
    F: Fn(&NativeNotification) -> StdResult<(), String>,
{
    if let Err(err) = sender(&note) {
        if verbose && !silent {
            eprintln!("visioflow: notification unavailable ({err})");
        }
    }
}

#[must_use]
pub fn should_notify_for_event(mode: CaptureNotify, event: &RoutingEvent) -> bool {
    match mode {
        CaptureNotify::Off => false,
        CaptureNotify::On => true,
        CaptureNotify::ErrorsOnly => matches!(
            event,
            RoutingEvent::Mismatch { .. } | RoutingEvent::NoAutoMatch
        ),
    }
}

#[must_use]
pub fn notification_title_for_event(_event: &RoutingEvent) -> String {
    "VisioFlow".to_owned()
}

#[must_use]
pub fn routing_notification_header(event: &RoutingEvent) -> String {
    match event {
        RoutingEvent::Matched {
            rule,
            auto_route: true,
        } => format!(r#"Matched rule "{rule}""#),
        RoutingEvent::Matched {
            rule,
            auto_route: false,
        } => format!(r#"Running rule "{rule}""#),
        RoutingEvent::Mismatch { rule } => format!(r#"Rule "{rule}" did not match"#),
        RoutingEvent::NoAutoMatch => "No rule matched".to_owned(),
        RoutingEvent::BuiltinCopy => "Copied to clipboard".to_owned(),
        RoutingEvent::BuiltinPlain => "Plain text mode".to_owned(),
    }
}

#[must_use]
pub fn routing_notification_body(event: &RoutingEvent, payload: &str) -> String {
    let header = routing_notification_header(event);
    let raw = truncate_for_toast(payload, TOAST_BODY_MAX_CHARS);
    format!("{header}\n\nRaw text:\n{raw}")
}

#[must_use]
pub fn build_routing_notification(
    event: &RoutingEvent,
    payload: &str,
    already_copied: bool,
) -> NativeNotification {
    NativeNotification {
        title: notification_title_for_event(event),
        body: routing_notification_body(event, payload),
        copy_payload: Some(payload.to_owned()),
        already_copied,
    }
}

#[must_use]
pub fn wifi_error_notification(
    err: &RuleError,
    rule_name: &str,
    payload: &str,
) -> Option<NativeNotification> {
    match err {
        RuleError::WifiConnectFailed(detail) => Some(NativeNotification {
            title: format!("VisioFlow WiFi ({rule_name})"),
            body: format!(
                "{} — {detail}",
                routing_notification_body(
                    &RoutingEvent::Matched {
                        rule: rule_name.to_owned(),
                        auto_route: false,
                    },
                    payload,
                )
            ),
            copy_payload: Some(payload.to_owned()),
            already_copied: false,
        }),
        _ => None,
    }
}

fn map_routing_error(err: RuleError) -> visioflow_core::VisioFlowError {
    let message = match err {
        RuleError::WifiConnectFailed(detail) => format!(
            "wifi connect failed: {detail}. Hint: verify location permission is enabled and WiFi control is allowed by your system policy."
        ),
        RuleError::ExecFailed(detail) => format!(
            "exec failed: {detail}. Hint: confirm the action path exists and is allowed by endpoint policy."
        ),
        other => other.to_string(),
    };
    visioflow_core::VisioFlowError::Capture(message)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcRoutingDecision {
    ExecuteMatchedRule { rule_name: String },
    HandleLocally,
}

pub fn decide_ipc_routing<S: RuleStore>(
    store: &S,
    payload: &str,
    mode: &RouteMode,
) -> RuleResult<IpcRoutingDecision> {
    if !matches!(mode, RouteMode::Auto(_)) {
        return Ok(IpcRoutingDecision::HandleLocally);
    }

    let route = visioflow_core::route_payload(store, mode.clone(), payload)?;
    let Some(routed) = route.routed else {
        return Ok(IpcRoutingDecision::HandleLocally);
    };
    Ok(IpcRoutingDecision::ExecuteMatchedRule {
        rule_name: routed.rule.name,
    })
}

/// Route the first captured payload through a named rule (regex + native parsers).
pub fn route_capture_trigger(
    store: &FileRuleStore,
    rule_name: &str,
    payloads: &[String],
) -> RuleResult<RoutedPayload> {
    let payload = payloads
        .first()
        .ok_or_else(|| RuleError::StoreIo("no payloads decoded for trigger".to_owned()))?;
    let engine = RuleEngine::new(store.clone());
    engine
        .route_fully(rule_name, payload)
        .map_err(|err| map_trigger_error(err, payloads, rule_name, store))
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

#[cfg(test)]
mod routing_tests {
    use super::*;
    use tempfile::TempDir;
    use visioflow_core::Rule;

    fn temp_store() -> (TempDir, FileRuleStore) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("rules.json");
        (dir, FileRuleStore::new(path))
    }

    #[test]
    fn route_mode_omitted_trigger_is_auto() {
        let mode = route_mode_from_trigger(None, &["wifi".to_owned()], &[]);
        let RouteMode::Auto(opts) = mode else {
            panic!("expected auto");
        };
        assert!(opts.except.contains("wifi"));
        assert_eq!(opts.only, None);
    }

    #[test]
    fn route_mode_copy_is_builtin() {
        assert_eq!(
            route_mode_from_trigger(Some("copy"), &[], &[]),
            RouteMode::BuiltinCopy
        );
    }

    #[test]
    fn format_routing_message_auto_matched() {
        let msg = format_capture_routing_message(
            &RoutingEvent::Matched {
                rule: "url".to_owned(),
                auto_route: true,
            },
            false,
        );
        assert_eq!(msg, r#"visioflow: matched rule "url""#);
    }

    #[test]
    fn route_payload_auto_picks_lowest_priority() {
        let (_dir, store) = temp_store();

        let mut url = Rule::new("url");
        url.auto_compatible = true;
        url.priority = 10;
        url.regex = Some(r"^https?://\S+$".to_owned());
        store.upsert(&url).expect("upsert");

        let mut plain = Rule::new("plain");
        plain.auto_compatible = true;
        plain.priority = 999;
        store.upsert(&plain).expect("upsert");

        let route = visioflow_core::route_payload(
            &store,
            RouteMode::Auto(visioflow_core::AutoRouteOptions::default()),
            "https://example.com",
        )
        .expect("route");
        let routed = route.routed.expect("matched");

        assert_eq!(routed.rule.name, "url");
    }

    #[test]
    fn route_payload_auto_except_skips_rule() {
        let (_dir, store) = temp_store();

        let mut wifi = Rule::new("wifi");
        wifi.auto_compatible = true;
        wifi.priority = 5;
        wifi.wifi_connect = true;
        store.upsert(&wifi).expect("upsert");

        let mut plain = Rule::new("plain");
        plain.auto_compatible = true;
        plain.priority = 999;
        store.upsert(&plain).expect("upsert");

        let routed = visioflow_core::route_payload(
            &store,
            RouteMode::Auto(visioflow_core::AutoRouteOptions {
                except: ["wifi".to_owned()].into_iter().collect(),
                only: None,
            }),
            "WIFI:T:WPA;S:lab;P:secret;;",
        )
        .expect("route")
        .routed
        .expect("matched");

        assert_eq!(routed.rule.name, "plain");
    }

    #[test]
    fn apply_routing_explicit_mismatch_copies_by_default() {
        let (_dir, store) = temp_store();
        let mut asset = Rule::new("asset");
        asset.regex = Some(r"ASSET:(?P<asset>\d+)".to_owned());
        store.upsert(&asset).expect("upsert");

        let result = apply_routing_after_halts(
            &store,
            &["https://example.com".to_owned()],
            RouteMode::Explicit("asset".to_owned()),
            OnMismatch::Copy,
            WifiHandoffMode::OpenSettings,
            CaptureNotify::Off,
            false,
            true,
        )
        .expect("routing");

        assert!(matches!(result, RoutingApplyResult::CopiedPayload { .. }));
    }

    #[test]
    fn apply_routing_sets_wifi_handoff_mode_env_for_wifi_payload() {
        let (_dir, store) = temp_store();
        let mut wifi = Rule::new("wifi");
        wifi.auto_compatible = true;
        wifi.priority = 5;
        wifi.regex = Some("^WIFI:".to_owned());
        store.upsert(&wifi).expect("upsert");

        let result = apply_routing_after_halts(
            &store,
            &["WIFI:T:WPA;S:lab;P:secret;;".to_owned()],
            RouteMode::Auto(visioflow_core::AutoRouteOptions::default()),
            OnMismatch::Copy,
            WifiHandoffMode::Print,
            CaptureNotify::Off,
            false,
            true,
        )
        .expect("routing");

        let RoutingApplyResult::Matched(routed) = result else {
            panic!("expected matched");
        };
        assert_eq!(
            routed.vars.get("VISIOFLOW_WIFI_HANDOFF_MODE"),
            Some("print")
        );
    }

    #[test]
    fn canonical_route_mode_uses_core_auto_sets() {
        let mode = route_mode_from_trigger(
            None,
            &["wifi".to_owned(), "wifi".to_owned()],
            &["url".to_owned()],
        );

        let visioflow_core::RouteMode::Auto(opts) = mode else {
            panic!("expected auto");
        };
        assert!(opts.except.contains("wifi"));
        assert_eq!(opts.except.len(), 1);
        assert_eq!(
            opts.only.as_ref().map(|only| only.contains("url")),
            Some(true)
        );
    }

    #[test]
    fn ipc_routing_decision_auto_route_chooses_matched_rule() {
        let (_dir, store) = temp_store();
        let mut url = Rule::new("url");
        url.auto_compatible = true;
        url.priority = 10;
        url.regex = Some(r"^https?://\S+$".to_owned());
        store.upsert(&url).expect("upsert");

        let mode = route_mode_from_trigger(None, &[], &[]);
        let decision = decide_ipc_routing(&store, "https://example.com", &mode).expect("decision");

        assert_eq!(
            decision,
            IpcRoutingDecision::ExecuteMatchedRule {
                rule_name: "url".to_owned(),
            }
        );
    }

    #[test]
    fn map_routing_error_adds_actionable_wifi_hint() {
        let err = map_routing_error(RuleError::WifiConnectFailed("missing ssid".to_owned()));
        let text = err.to_string();
        assert!(text.contains("location permission"));
        assert!(text.contains("system policy"));
    }

    #[test]
    fn notify_errors_only_skips_successful_match() {
        let should_notify = should_notify_for_event(
            CaptureNotify::ErrorsOnly,
            &RoutingEvent::Matched {
                rule: "url".to_owned(),
                auto_route: true,
            },
        );
        assert!(!should_notify);
    }

    #[test]
    fn notify_errors_only_includes_no_auto_match() {
        let should_notify =
            should_notify_for_event(CaptureNotify::ErrorsOnly, &RoutingEvent::NoAutoMatch);
        assert!(should_notify);
    }

    #[test]
    fn routing_notification_title_is_always_visioflow() {
        let note = build_routing_notification(
            &RoutingEvent::Matched {
                rule: "url".to_owned(),
                auto_route: true,
            },
            "https://example.com",
            false,
        );
        assert_eq!(note.title, "VisioFlow");
    }

    #[test]
    fn routing_notification_body_auto_matched_rule() {
        let payload = "https://www.google.com";
        let note = build_routing_notification(
            &RoutingEvent::Matched {
                rule: "url".to_owned(),
                auto_route: true,
            },
            payload,
            false,
        );
        assert_eq!(
            note.body,
            "Matched rule \"url\"\n\nRaw text:\nhttps://www.google.com"
        );
    }

    #[test]
    fn routing_notification_body_explicit_matched_rule() {
        let payload = "MATMSG:TO:user@example.com;SUB:Hello;";
        let note = build_routing_notification(
            &RoutingEvent::Matched {
                rule: "plain".to_owned(),
                auto_route: false,
            },
            payload,
            false,
        );
        assert_eq!(
            note.body,
            "Running rule \"plain\"\n\nRaw text:\nMATMSG:TO:user@example.com;SUB:Hello;"
        );
    }

    #[test]
    fn routing_notification_body_wifi_auto_matched() {
        let payload = "WIFI:T:WPA;S:lab;P:secret;;";
        let note = build_routing_notification(
            &RoutingEvent::Matched {
                rule: "wifi".to_owned(),
                auto_route: true,
            },
            payload,
            false,
        );
        assert_eq!(
            note.body,
            "Matched rule \"wifi\"\n\nRaw text:\nWIFI:T:WPA;S:lab;P:secret;;"
        );
    }

    #[test]
    fn build_routing_notification_includes_full_copy_payload() {
        let payload = "x".repeat(300);
        let note = build_routing_notification(
            &RoutingEvent::Matched {
                rule: "url".to_owned(),
                auto_route: true,
            },
            &payload,
            false,
        );
        assert_eq!(note.copy_payload.as_deref(), Some(payload.as_str()));
    }

    #[test]
    fn routing_notification_body_truncates_only_raw_payload() {
        let payload = "x".repeat(300);
        let body = routing_notification_body(
            &RoutingEvent::Matched {
                rule: "url".to_owned(),
                auto_route: true,
            },
            &payload,
        );
        assert!(body.starts_with("Matched rule \"url\"\n\nRaw text:\n"));
        let raw_line = body.strip_prefix("Matched rule \"url\"\n\nRaw text:\n").unwrap();
        assert!(raw_line.ends_with('…'));
        assert_eq!(raw_line.chars().count(), 257);
    }

    #[test]
    fn routing_notification_body_builtin_copy() {
        let note = build_routing_notification(&RoutingEvent::BuiltinCopy, "scan-me", true);
        assert_eq!(
            note.body,
            "Copied to clipboard\n\nRaw text:\nscan-me"
        );
    }

    #[test]
    fn build_routing_notification_copy_again_when_already_copied() {
        let note = build_routing_notification(&RoutingEvent::BuiltinCopy, "scan-me", true);
        assert!(note.already_copied);
        assert_eq!(
            crate::notifications::toast_copy_action_label(note.already_copied),
            "Copy again"
        );
    }

    #[test]
    fn build_routing_notification_copy_when_not_auto_copied() {
        let note = build_routing_notification(
            &RoutingEvent::Matched {
                rule: "url".to_owned(),
                auto_route: true,
            },
            "https://example.com",
            false,
        );
        assert!(!note.already_copied);
        assert_eq!(
            crate::notifications::toast_copy_action_label(note.already_copied),
            "Copy"
        );
    }

    #[test]
    fn routing_notification_body_no_auto_match() {
        let note = build_routing_notification(&RoutingEvent::NoAutoMatch, "unknown-payload", false);
        assert_eq!(
            note.body,
            "No rule matched\n\nRaw text:\nunknown-payload"
        );
    }

    #[test]
    fn notify_then_action_runs_notify_before_action() {
        use std::sync::atomic::{AtomicU8, Ordering};

        static ORDER: AtomicU8 = AtomicU8::new(0);
        ORDER.store(0, Ordering::SeqCst);

        let sender = |_: &NativeNotification| {
            assert_eq!(ORDER.fetch_add(1, Ordering::SeqCst), 0);
            Ok(())
        };
        let action = || {
            assert_eq!(ORDER.fetch_add(1, Ordering::SeqCst), 1);
            Ok(())
        };

        notify_then_action(
            CaptureNotify::On,
            &RoutingEvent::Matched {
                rule: "wifi".to_owned(),
                auto_route: true,
            },
            "WIFI:T:WPA;S:lab;P:secret;;",
            false,
            true,
            sender,
            action,
        )
        .expect("sequenced");

        assert_eq!(ORDER.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn notify_then_action_skips_notify_when_disabled() {
        use std::sync::atomic::{AtomicU8, Ordering};

        static ORDER: AtomicU8 = AtomicU8::new(0);
        ORDER.store(0, Ordering::SeqCst);

        let sender = |_: &NativeNotification| {
            ORDER.fetch_add(1, Ordering::SeqCst);
            Ok(())
        };

        notify_then_action(
            CaptureNotify::Off,
            &RoutingEvent::Matched {
                rule: "wifi".to_owned(),
                auto_route: true,
            },
            "payload",
            false,
            true,
            sender,
            || Ok(()),
        )
        .expect("action only");

        assert_eq!(ORDER.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn routing_notification_text_for_mismatch_uses_payload() {
        let payload = "https://example.com";
        let note = build_routing_notification(
            &RoutingEvent::Mismatch {
                rule: "asset".to_owned(),
            },
            payload,
            false,
        );
        assert_eq!(note.title, "VisioFlow");
        assert_eq!(
            note.body,
            "Rule \"asset\" did not match\n\nRaw text:\nhttps://example.com"
        );
    }

    #[test]
    fn wifi_connect_failure_maps_to_notification() {
        let payload = "WIFI:T:WPA;S:lab;P:secret;;";
        let maybe = wifi_error_notification(
            &RuleError::WifiConnectFailed("access denied".to_owned()),
            "wifi",
            payload,
        );
        assert!(maybe.is_some());
        let note = maybe.expect("notification");
        assert!(note.title.contains("wifi"));
        assert!(note.body.contains("access denied"));
        assert!(note.body.contains(payload));
    }
}

#[cfg(test)]
mod trigger_tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;
    use visioflow_core::{
        apply_rule, merge_native_vars, resolve_payload_fully, ResolvedVars, Rule, RuleStore,
    };

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

        let routed =
            route_capture_trigger(&store, "asset", &["ASSET:7".to_owned(), "skip".to_owned()])
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
    fn spawn_rule_actions_passes_env_to_child() {
        let dir = TempDir::new().expect("tempdir");
        let out_path = dir.path().join("child-out.txt");
        let script_path = write_env_echo_script(&dir, &out_path);

        let mut rule = Rule::new("run");
        rule.exec = Some(script_path);

        let mut vars = ResolvedVars::new();
        vars.insert("QR_VAR_ASSET", "triggered-99");

        spawn_rule_actions(&rule, &vars).expect("spawn");

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
