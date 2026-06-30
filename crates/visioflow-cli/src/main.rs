use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use visioflow_cli::commands::capture::{
    apply_capture_halts, apply_routing_after_halts, decide_ipc_routing, notify_routing_outcome,
    route_mode_from_trigger, run_capture, write_capture_output, CaptureAction, CaptureArgs,
    CaptureFilter, CaptureNotify, CaptureSource, ExposureBracketMode, IpcRoutingDecision,
    OnMismatch, PreviewPosition, RoutingApplyResult, WifiHandoffMode,
};
use visioflow_cli::commands::daemon::{
    daemon_reload_ipc, daemon_start, daemon_status, daemon_stop, default_pid_path,
    route_capture_trigger_via_ipc, rule_execute_via_ipc, run_daemon_server_loop,
};
use visioflow_cli::commands::exec::spawn_rule_actions;
use visioflow_cli::commands::notify::{notify_copy_from_toast, notify_test};
use visioflow_cli::commands::rule::{
    map_rule_error, open_store, rule_config, rule_create, rule_delete, rule_execute,
    rule_init_defaults, rule_list, rule_set_action, write_resolved_output, write_rule_list_output,
    RuleOutputFormat,
};
#[cfg(feature = "opencv-webcam")]
use visioflow_cli::webcam_preview::DEFAULT_DECODE_INTERVAL_MS;
#[cfg(feature = "opencv-webcam")]
use visioflow_cli::webcam_session::{DEFAULT_EXPOSURE_FLUSH_GRABS, DEFAULT_EXPOSURE_STEP_MS};

#[cfg(not(feature = "opencv-webcam"))]
const DEFAULT_DECODE_INTERVAL_MS: u64 = 100;
#[cfg(not(feature = "opencv-webcam"))]
const DEFAULT_EXPOSURE_FLUSH_GRABS: u32 = 2;
#[cfg(not(feature = "opencv-webcam"))]
const DEFAULT_EXPOSURE_STEP_MS: u64 = 100;
use visioflow_core::default_socket_path;
use visioflow_core::RoutingEvent;

#[derive(Debug, Parser)]
#[command(
    name = "visioflow",
    about = "Optical automation engine for visual payload routing"
)]
struct Cli {
    #[arg(long, value_enum, default_value = "plain")]
    output: OutputFormat,

    #[arg(long, short, global = true)]
    verbose: bool,

    #[arg(long, global = true)]
    silent: bool,

    #[arg(long, value_enum, global = true)]
    export: Option<ExportFormat>,

    #[arg(long, global = true)]
    ipc_socket: Option<String>,

    /// Air-gap mode: blocks network telemetry (OTLP); hidden until telemetry ships
    #[arg(long, global = true, hide = true)]
    disable_telemetry: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Plain,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ExportFormat {
    Bash,
    Ps1,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Manage routing rules
    Rule {
        /// Path to rules JSON store (integration tests only)
        #[arg(long, hide = true)]
        store: Option<PathBuf>,

        #[command(subcommand)]
        command: RuleCommands,
    },

    /// Capture and decode visual payloads
    Capture {
        #[arg(long, value_enum)]
        source: CaptureSource,

        #[arg(long, value_enum, default_value = "otsu")]
        filter: CaptureFilter,

        #[arg(long, value_enum)]
        action: Option<CaptureAction>,

        /// Load frame from image file instead of a live source (integration tests)
        #[arg(long, hide = true)]
        input_image: Option<std::path::PathBuf>,

        /// Seconds to scan the webcam with live preview (webcam source only)
        #[arg(long, default_value_t = 20)]
        timeout: u64,

        /// Preview window anchor position (webcam source only)
        #[arg(long, value_enum, default_value = "bottom-center")]
        preview_position: PreviewPosition,

        /// Preview size as screen-height ratio (webcam source only)
        #[arg(long, default_value_t = 0.12_f32)]
        preview_scale: f32,

        /// Milliseconds to hold each exposure before advancing bracket (webcam only)
        #[arg(long, default_value_t = DEFAULT_EXPOSURE_STEP_MS)]
        exposure_step_ms: u64,

        /// Frames to discard after each exposure change (webcam only)
        #[arg(long, default_value_t = DEFAULT_EXPOSURE_FLUSH_GRABS)]
        exposure_flush_grabs: u32,

        /// Milliseconds between QR decode attempts (webcam only)
        #[arg(long, default_value_t = DEFAULT_DECODE_INTERVAL_MS)]
        decode_interval_ms: u64,

        /// Exposure bracket cycling: auto probes camera, on forces bracketing, off keeps auto only
        #[arg(long, value_enum, default_value = "auto")]
        exposure_bracket: ExposureBracketMode,

        /// Apply a named routing rule to the captured payload (omit for auto-route)
        #[arg(long)]
        trigger: Option<String>,

        /// Exclude rule(s) from auto-routing scan (repeatable)
        #[arg(long)]
        except: Vec<String>,

        /// Only consider these rules during auto-routing (repeatable)
        #[arg(long)]
        only: Vec<String>,

        /// Fallback when routing fails: copy payload or exit strict
        #[arg(long, value_enum, default_value = "copy")]
        on_mismatch: OnMismatch,

        /// WiFi QR action mode: open settings handoff UI or print credentials
        #[arg(long, value_enum, default_value = "open-settings")]
        wifi_handoff: WifiHandoffMode,

        /// Desktop notifications for routing outcomes
        #[arg(long, default_value_t = false)]
        notify: bool,

        /// Interactive list when multiple payloads are decoded
        #[arg(long)]
        select: bool,

        /// Confirm payload on stdin before action/trigger
        #[arg(long)]
        interactive: bool,

        /// Disable horizontal mirroring of the webcam preview (mirrored by default, selfie-style)
        #[arg(long)]
        no_mirror: bool,

        /// Path to rules JSON store (integration tests only)
        #[arg(long, hide = true)]
        store: Option<PathBuf>,
    },

    /// Desktop notification utilities
    Notify {
        #[command(subcommand)]
        command: NotifyCommands,
    },

    /// Background routing daemon
    Daemon {
        /// Path to rules JSON store (integration tests only)
        #[arg(long, hide = true)]
        store: Option<PathBuf>,

        /// IPC socket path (defaults to platform convention)
        #[arg(long)]
        socket: Option<String>,

        #[command(subcommand)]
        command: DaemonCommands,
    },
}

#[derive(Debug, Subcommand)]
enum NotifyCommands {
    /// Show a sample Windows toast without capture or webcam
    Test {
        #[arg(long, default_value = "VisioFlow")]
        title: String,

        #[arg(long, default_value = "Toast delivery smoke test")]
        body: String,

        /// Force a specific backend: winrt, powershell, or burnttoast
        #[arg(long, value_name = "BACKEND")]
        backend: Option<String>,
    },

    /// Copy a staged toast payload to the clipboard (invoked by toast Copy button)
    #[command(hide = true)]
    Copy {
        #[arg(long, value_name = "PATH")]
        from_toast: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum DaemonCommands {
    /// Start the daemon (foreground unless --hidden)
    Start {
        /// Run detached in the background
        #[arg(long)]
        hidden: bool,
    },

    /// Stop the running daemon
    Stop,

    /// Show daemon status
    Status,

    /// Reload rules from disk into the daemon
    Reload,
}

#[derive(Debug, Subcommand)]
enum RuleCommands {
    /// Create a new empty rule
    Create { name: String },

    /// Configure regex and capture mappings for a rule
    Config {
        name: String,

        #[arg(long)]
        regex: Option<String>,

        #[arg(long)]
        map: Vec<String>,
    },

    /// Set post-route actions for a rule (exec script and/or WiFi connect)
    SetAction {
        name: String,

        #[arg(long)]
        exec: Option<PathBuf>,

        /// Connect to WiFi using QR_NATIVE_WIFI_* vars after routing
        #[arg(long)]
        wifi_connect: bool,
    },

    /// Apply a rule to a payload and print resolved variables
    Execute {
        name: String,

        #[arg(long)]
        payload: String,

        /// Resolve and print variables without spawning the rule exec action
        #[arg(long)]
        no_exec: bool,
    },

    /// List all rules in the store
    List,

    /// Remove a rule from the store
    Delete { name: String },

    /// Install stock default routing rules
    InitDefaults {
        /// Add missing default rules only; do not overwrite existing names
        #[arg(long)]
        merge: bool,

        /// Replace the entire rule store with stock defaults
        #[arg(long)]
        force: bool,
    },
}

fn main() {
    if let Some(result) = visioflow_cli::notifications::try_dispatch_toast_protocol_activation() {
        match result {
            Ok(()) => return,
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }

    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn write_export_output(
    vars: &std::collections::HashMap<String, String>,
    format: ExportFormat,
    silent: bool,
) -> visioflow_core::error::Result<()> {
    let lines = match format {
        ExportFormat::Bash => visioflow_core::emit_bash(vars),
        ExportFormat::Ps1 => visioflow_core::emit_ps1(vars),
    };
    if !silent && !lines.is_empty() {
        print!("{lines}");
    }
    Ok(())
}

fn run() -> visioflow_core::error::Result<()> {
    let cli = Cli::parse();

    visioflow_core::enforce_airgap_policy(cli.disable_telemetry)?;

    if cli.verbose && !cli.silent {
        eprintln!("visioflow starting");
    }

    let ipc_socket = cli
        .ipc_socket
        .as_deref()
        .map(str::to_owned)
        .or_else(|| std::env::var("VISIOFLOW_IPC_SOCKET").ok());

    let output_format = match cli.output {
        OutputFormat::Plain => RuleOutputFormat::Plain,
        OutputFormat::Json => RuleOutputFormat::Json,
    };

    match cli.command {
        Commands::Rule { store, command } => {
            let store = open_store(store);
            match command {
                RuleCommands::Create { name } => {
                    rule_create(&store, &name).map_err(map_rule_error)?;
                }
                RuleCommands::Config { name, regex, map } => {
                    rule_config(&store, &name, regex.as_deref(), &map).map_err(map_rule_error)?;
                }
                RuleCommands::SetAction {
                    name,
                    exec,
                    wifi_connect,
                } => {
                    rule_set_action(&store, &name, exec.as_deref(), wifi_connect)
                        .map_err(map_rule_error)?;
                }
                RuleCommands::Execute {
                    name,
                    payload,
                    no_exec,
                } => {
                    let resolved = if let Some(ref socket) = ipc_socket {
                        rule_execute_via_ipc(socket, &name, &payload)?
                    } else {
                        let routed =
                            rule_execute(&store, &name, &payload).map_err(map_rule_error)?;
                        if !no_exec {
                            spawn_rule_actions(&routed.rule, &routed.vars)
                                .map_err(map_rule_error)?;
                        }
                        routed.vars
                    };
                    if let Some(export_fmt) = cli.export {
                        let vars = visioflow_core::vars_from_resolved(&resolved);
                        write_export_output(&vars, export_fmt, cli.silent)?;
                    } else {
                        write_resolved_output(&resolved, output_format, cli.silent)?;
                    }
                }
                RuleCommands::List => {
                    let rules = rule_list(&store).map_err(map_rule_error)?;
                    write_rule_list_output(&rules, output_format, cli.silent)?;
                }
                RuleCommands::Delete { name } => {
                    rule_delete(&store, &name).map_err(map_rule_error)?;
                }
                RuleCommands::InitDefaults { merge, force } => {
                    rule_init_defaults(&store, merge, force).map_err(map_rule_error)?;
                }
            }
        }
        Commands::Notify { command } => match command {
            NotifyCommands::Test {
                title,
                body,
                backend,
            } => {
                notify_test(
                    Some(&title),
                    Some(&body),
                    backend.as_deref(),
                    cli.verbose,
                )?;
            }
            NotifyCommands::Copy { from_toast } => {
                notify_copy_from_toast(&from_toast, cli.silent)?;
            }
        }
        Commands::Daemon {
            store,
            socket,
            command,
        } => {
            let socket_path = socket.unwrap_or_else(default_socket_path);
            let file_store = open_store(store);
            match command {
                DaemonCommands::Start { hidden } => {
                    if hidden {
                        daemon_start(
                            Some(&socket_path),
                            true,
                            Some(file_store.path().to_path_buf()),
                            cli.verbose,
                        )?;
                    } else {
                        run_daemon_server_loop(
                            &socket_path,
                            file_store,
                            &default_pid_path(),
                            cli.verbose,
                        )?;
                    }
                }
                DaemonCommands::Stop => {
                    daemon_stop(&default_pid_path())?;
                    if !cli.silent {
                        println!("daemon stopped");
                    }
                }
                DaemonCommands::Status => {
                    daemon_status(&socket_path, &default_pid_path())?;
                }
                DaemonCommands::Reload => {
                    daemon_reload_ipc(&socket_path)?;
                    if !cli.silent {
                        println!("daemon rules reloaded");
                    }
                }
            }
        }
        Commands::Capture {
            source,
            filter,
            action,
            input_image,
            timeout,
            preview_position,
            preview_scale,
            exposure_step_ms,
            exposure_flush_grabs,
            decode_interval_ms,
            exposure_bracket,
            trigger,
            except,
            only,
            on_mismatch,
            wifi_handoff,
            notify,
            select,
            interactive,
            store: rule_store,
            no_mirror,
        } => {
            let payloads = run_capture(CaptureArgs {
                source,
                filter,
                action,
                input_image,
                timeout_secs: timeout,
                verbose: cli.verbose,
                preview_position,
                preview_scale,
                exposure_step_ms,
                exposure_flush_grabs,
                decode_interval_ms,
                exposure_bracket,
                trigger: trigger.clone(),
                except: except.clone(),
                only: only.clone(),
                on_mismatch,
                wifi_handoff,
                notify,
                rule_store: rule_store.clone(),
                select,
                interactive,
                mirror: !no_mirror,
            })?;

            let mut stdin = std::io::stdin().lock();
            let mut stderr = std::io::stderr().lock();
            let payloads =
                apply_capture_halts(payloads, select, interactive, &mut stdin, &mut stderr)?;

            let routing_active = trigger.is_some() || action.is_none();

            if routing_active {
                if cli.verbose && !cli.silent {
                    if let Some(rule_name) = trigger.as_deref() {
                        eprintln!(
                            "decoded {} payload(s) for trigger '{rule_name}'",
                            payloads.len()
                        );
                    } else {
                        eprintln!("decoded {} payload(s) for auto-route", payloads.len());
                    }
                    for (index, payload) in payloads.iter().enumerate() {
                        eprintln!("  [{index}] {payload:?}");
                    }
                }

                let store = open_store(rule_store.clone());
                let mode = route_mode_from_trigger(trigger.as_deref(), &except, &only);
                let explicit_ipc_trigger = trigger
                    .as_deref()
                    .filter(|name| !visioflow_core::is_builtin_trigger(name) && *name != "auto");

                let ipc_decision =
                    if let Some(rule_name) = explicit_ipc_trigger {
                        Some(IpcRoutingDecision::ExecuteMatchedRule {
                            rule_name: rule_name.to_owned(),
                        })
                    } else if ipc_socket.is_some() {
                        let payload = payloads.first().ok_or_else(|| {
                            visioflow_core::VisioFlowError::Capture(
                                "no payloads decoded for routing".into(),
                            )
                        })?;
                        Some(decide_ipc_routing(&store, payload, &mode).map_err(|err| {
                            visioflow_core::VisioFlowError::Capture(err.to_string())
                        })?)
                    } else {
                        None
                    };

                if let (Some(socket), Some(IpcRoutingDecision::ExecuteMatchedRule { rule_name })) =
                    (ipc_socket.as_ref(), ipc_decision)
                {
                    let payload = payloads.first().ok_or_else(|| {
                        visioflow_core::VisioFlowError::Capture(
                            "no payloads decoded for routing".into(),
                        )
                    })?;
                    let notify_mode = if notify {
                        CaptureNotify::On
                    } else {
                        CaptureNotify::Off
                    };
                    notify_routing_outcome(
                        notify_mode,
                        &RoutingEvent::Matched {
                            rule: rule_name.clone(),
                            auto_route: explicit_ipc_trigger.is_none(),
                        },
                        payload,
                        cli.verbose,
                        cli.silent,
                    );
                    let vars = route_capture_trigger_via_ipc(socket, &rule_name, &payloads)?;
                    if let Some(export_fmt) = cli.export {
                        let map = visioflow_core::vars_from_resolved(&vars);
                        write_export_output(&map, export_fmt, cli.silent)?;
                    } else if matches!(output_format, RuleOutputFormat::Json) {
                        write_resolved_output(&vars, RuleOutputFormat::Json, cli.silent)?;
                    } else if matches!(action, Some(CaptureAction::Stdout)) || trigger.is_some() {
                        write_resolved_output(&vars, RuleOutputFormat::Plain, cli.silent)?;
                    }
                } else {
                    let notify_mode = if notify {
                        CaptureNotify::On
                    } else {
                        CaptureNotify::Off
                    };
                    let result = apply_routing_after_halts(
                        &store,
                        &payloads,
                        mode,
                        on_mismatch,
                        wifi_handoff,
                        notify_mode,
                        cli.verbose,
                        cli.silent,
                    )?;

                    match result {
                        RoutingApplyResult::Matched(routed) => {
                            if let Some(export_fmt) = cli.export {
                                let vars = visioflow_core::vars_from_resolved(&routed.vars);
                                write_export_output(&vars, export_fmt, cli.silent)?;
                            } else if matches!(output_format, RuleOutputFormat::Json) {
                                write_resolved_output(
                                    &routed.vars,
                                    RuleOutputFormat::Json,
                                    cli.silent,
                                )?;
                            } else if matches!(action, Some(CaptureAction::Stdout))
                                || trigger.is_some()
                            {
                                write_resolved_output(
                                    &routed.vars,
                                    RuleOutputFormat::Plain,
                                    cli.silent,
                                )?;
                            }
                        }
                        RoutingApplyResult::CopiedPayload { .. }
                        | RoutingApplyResult::PrintedPayload(_) => {
                            if let Some(export_fmt) = cli.export {
                                let vars = visioflow_core::vars_from_payloads(&payloads);
                                write_export_output(&vars, export_fmt, cli.silent)?;
                            }
                        }
                    }
                }
            } else if let Some(export_fmt) = cli.export {
                let vars = visioflow_core::vars_from_payloads(&payloads);
                write_export_output(&vars, export_fmt, cli.silent)?;
            } else if matches!(output_format, RuleOutputFormat::Json) {
                let json = serde_json::to_string(&payloads).map_err(|e| {
                    visioflow_core::VisioFlowError::Capture(format!("json encode failed: {e}"))
                })?;
                if !cli.silent {
                    println!("{json}");
                }
            } else {
                write_capture_output(
                    &payloads,
                    action.unwrap_or(CaptureAction::Stdout),
                    cli.silent,
                )?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};

    #[test]
    fn capture_defaults_preview_position_and_scale() {
        let cli = Cli::try_parse_from([
            "visioflow",
            "capture",
            "--source",
            "webcam",
            "--action",
            "stdout",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Capture {
                preview_position,
                preview_scale,
                exposure_step_ms,
                exposure_flush_grabs,
                decode_interval_ms,
                exposure_bracket,
                ..
            } => {
                assert!(matches!(preview_position, PreviewPosition::BottomCenter));
                assert!((preview_scale - 0.12).abs() < f32::EPSILON);
                assert_eq!(exposure_step_ms, DEFAULT_EXPOSURE_STEP_MS);
                assert_eq!(exposure_flush_grabs, DEFAULT_EXPOSURE_FLUSH_GRABS);
                assert_eq!(decode_interval_ms, DEFAULT_DECODE_INTERVAL_MS);
                assert!(matches!(exposure_bracket, ExposureBracketMode::Auto));
            }
            Commands::Rule { .. } | Commands::Daemon { .. } | Commands::Notify { .. } => {}
        }
    }

    #[test]
    fn capture_accepts_trigger_flag() {
        let cli = Cli::try_parse_from([
            "visioflow",
            "capture",
            "--source",
            "snip",
            "--action",
            "stdout",
            "--trigger",
            "asset",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Capture { trigger, .. } => {
                assert_eq!(trigger.as_deref(), Some("asset"));
            }
            Commands::Rule { .. } | Commands::Daemon { .. } | Commands::Notify { .. } => {
                panic!("expected capture")
            }
        }
    }

    #[test]
    fn capture_accepts_custom_preview_options() {
        let cli = Cli::try_parse_from([
            "visioflow",
            "capture",
            "--source",
            "webcam",
            "--action",
            "stdout",
            "--preview-position",
            "top-right",
            "--preview-scale",
            "0.25",
            "--exposure-step-ms",
            "150",
            "--exposure-flush-grabs",
            "2",
            "--decode-interval-ms",
            "200",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Capture {
                preview_position,
                preview_scale,
                exposure_step_ms,
                exposure_flush_grabs,
                decode_interval_ms,
                ..
            } => {
                assert!(matches!(preview_position, PreviewPosition::TopRight));
                assert!((preview_scale - 0.25).abs() < f32::EPSILON);
                assert_eq!(exposure_step_ms, 150);
                assert_eq!(exposure_flush_grabs, 2);
                assert_eq!(decode_interval_ms, 200);
            }
            Commands::Rule { .. } | Commands::Daemon { .. } | Commands::Notify { .. } => {}
        }
    }

    #[test]
    fn capture_accepts_select_and_interactive_flags() {
        let cli = Cli::try_parse_from([
            "visioflow",
            "capture",
            "--source",
            "snip",
            "--action",
            "stdout",
            "--select",
            "--interactive",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Capture {
                select,
                interactive,
                ..
            } => {
                assert!(select);
                assert!(interactive);
            }
            Commands::Rule { .. } | Commands::Daemon { .. } | Commands::Notify { .. } => {
                panic!("expected capture")
            }
        }
    }

    #[test]
    fn cli_parses_hidden_disable_telemetry_flag() {
        let cli = Cli::try_parse_from([
            "visioflow",
            "--disable-telemetry",
            "capture",
            "--source",
            "snip",
            "--action",
            "stdout",
        ])
        .expect("cli should parse");

        assert!(cli.disable_telemetry);
    }

    #[test]
    fn capture_accepts_routing_flags() {
        let cli = Cli::try_parse_from([
            "visioflow",
            "capture",
            "--source",
            "snip",
            "--except",
            "wifi",
            "--except",
            "asset",
            "--only",
            "url",
            "--on-mismatch",
            "none",
            "--wifi-handoff",
            "print",
            "--notify",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Capture {
                action,
                trigger,
                except,
                only,
                on_mismatch,
                wifi_handoff,
                notify,
                ..
            } => {
                assert!(action.is_none());
                assert!(trigger.is_none());
                assert_eq!(except, vec!["wifi", "asset"]);
                assert_eq!(only, vec!["url"]);
                assert!(matches!(on_mismatch, OnMismatch::None));
                assert!(matches!(wifi_handoff, WifiHandoffMode::Print));
                assert!(notify);
            }
            Commands::Rule { .. } | Commands::Daemon { .. } | Commands::Notify { .. } => {
                panic!("expected capture")
            }
        }
    }

    #[test]
    fn capture_notify_defaults_to_false() {
        let cli = Cli::try_parse_from(["visioflow", "capture", "--source", "snip"])
            .expect("cli should parse");

        match cli.command {
            Commands::Capture { notify, .. } => assert!(!notify),
            Commands::Rule { .. } | Commands::Daemon { .. } | Commands::Notify { .. } => {
                panic!("expected capture")
            }
        }
    }

    #[test]
    fn capture_help_mentions_notify_flag() {
        let mut root = Cli::command();
        let help = root
            .find_subcommand_mut("capture")
            .expect("capture command")
            .render_help()
            .to_string();
        assert!(help.contains("--notify"));
    }

    #[test]
    fn capture_webcam_mirrors_by_default() {
        let cli = Cli::try_parse_from([
            "visioflow",
            "capture",
            "--source",
            "webcam",
            "--action",
            "stdout",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Capture { no_mirror, .. } => assert!(!no_mirror),
            Commands::Rule { .. } | Commands::Daemon { .. } | Commands::Notify { .. } => {
                panic!("expected capture")
            }
        }
    }

    #[test]
    fn capture_accepts_no_mirror_flag() {
        let cli = Cli::try_parse_from([
            "visioflow",
            "capture",
            "--source",
            "webcam",
            "--action",
            "stdout",
            "--no-mirror",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Capture { no_mirror, .. } => assert!(no_mirror),
            Commands::Rule { .. } | Commands::Daemon { .. } | Commands::Notify { .. } => {
                panic!("expected capture")
            }
        }
    }

    #[test]
    fn capture_help_mentions_no_mirror_flag() {
        let mut root = Cli::command();
        let help = root
            .find_subcommand_mut("capture")
            .expect("capture command")
            .render_help()
            .to_string();
        assert!(help.contains("--no-mirror"));
    }

    #[test]
    fn notify_test_command_parses() {
        let cli = Cli::try_parse_from([
            "visioflow",
            "notify",
            "test",
            "--title",
            "T",
            "--body",
            "B",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Notify {
                command: NotifyCommands::Test {
                    title,
                    body,
                    backend: _,
                },
            } => {
                assert_eq!(title, "T");
                assert_eq!(body, "B");
            }
            Commands::Rule { .. } | Commands::Capture { .. } | Commands::Daemon { .. } => {
                panic!("expected notify")
            }
            Commands::Notify {
                command: NotifyCommands::Copy { .. },
            } => panic!("expected notify test"),
        }
    }

    #[test]
    fn notify_copy_command_parses() {
        let cli = Cli::try_parse_from([
            "visioflow",
            "notify",
            "copy",
            "--from-toast",
            r"C:\Users\me\AppData\Local\Temp\visioflow-toast-copy-1-0.txt",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Notify {
                command: NotifyCommands::Copy { from_toast },
            } => {
                assert!(from_toast
                    .to_string_lossy()
                    .contains("visioflow-toast-copy-1-0.txt"));
            }
            _ => panic!("expected notify copy"),
        }
    }

    #[test]
    fn airgap_policy_blocks_run_when_flag_set() {
        let err = visioflow_core::enforce_airgap_policy(true).expect_err("expected air-gap");
        assert!(matches!(err, visioflow_core::VisioFlowError::AirGap));
    }
}
