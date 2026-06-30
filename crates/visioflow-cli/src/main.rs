use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use visioflow_cli::commands::capture::{
    route_capture_trigger, run_capture, spawn_rule_exec, write_capture_output, CaptureAction,
    CaptureArgs, CaptureFilter, CaptureSource, ExposureBracketMode, PreviewPosition,
};
use visioflow_cli::commands::daemon::{
    daemon_reload_ipc, daemon_start, daemon_status, daemon_stop, default_pid_path,
    route_capture_trigger_via_ipc, rule_execute_via_ipc, run_daemon_server_loop,
};
use visioflow_cli::commands::rule::{
    map_rule_error, open_store, rule_config, rule_create, rule_execute, rule_set_action,
    write_resolved_output, RuleOutputFormat,
};
#[cfg(feature = "opencv-webcam")]
use visioflow_cli::webcam_preview::DEFAULT_DECODE_INTERVAL_MS;
#[cfg(feature = "opencv-webcam")]
use visioflow_cli::webcam_session::{
    DEFAULT_EXPOSURE_FLUSH_GRABS, DEFAULT_EXPOSURE_STEP_MS,
};

#[cfg(not(feature = "opencv-webcam"))]
const DEFAULT_DECODE_INTERVAL_MS: u64 = 100;
#[cfg(not(feature = "opencv-webcam"))]
const DEFAULT_EXPOSURE_FLUSH_GRABS: u32 = 2;
#[cfg(not(feature = "opencv-webcam"))]
const DEFAULT_EXPOSURE_STEP_MS: u64 = 100;
use visioflow_core::default_socket_path;

#[derive(Debug, Parser)]
#[command(name = "visioflow", about = "Optical automation engine for visual payload routing")]
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
        action: CaptureAction,

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

        /// Apply a named routing rule to the captured payload
        #[arg(long)]
        trigger: Option<String>,

        /// Path to rules JSON store (integration tests only)
        #[arg(long, hide = true)]
        store: Option<PathBuf>,
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
    Create {
        name: String,
    },

    /// Configure regex and capture mappings for a rule
    Config {
        name: String,

        #[arg(long)]
        regex: Option<String>,

        #[arg(long)]
        map: Vec<String>,
    },

    /// Set the executable action for a rule
    SetAction {
        name: String,

        #[arg(long)]
        exec: PathBuf,
    },

    /// Apply a rule to a payload and print resolved variables
    Execute {
        name: String,

        #[arg(long)]
        payload: String,
    },
}

fn main() {
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
                    rule_config(
                        &store,
                        &name,
                        regex.as_deref(),
                        &map,
                    )
                    .map_err(map_rule_error)?;
                }
                RuleCommands::SetAction { name, exec } => {
                    rule_set_action(&store, &name, &exec).map_err(map_rule_error)?;
                }
                RuleCommands::Execute { name, payload } => {
                    let resolved = if let Some(ref socket) = ipc_socket {
                        rule_execute_via_ipc(socket, &name, &payload)?
                    } else {
                        rule_execute(&store, &name, &payload).map_err(map_rule_error)?
                    };
                    if let Some(export_fmt) = cli.export {
                        let vars = visioflow_core::vars_from_resolved(&resolved);
                        write_export_output(&vars, export_fmt, cli.silent)?;
                    } else {
                        write_resolved_output(&resolved, output_format, cli.silent)?;
                    }
                }
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
            store: rule_store,
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
                rule_store: rule_store.clone(),
            })?;

            if let Some(rule_name) = trigger {
                if cli.verbose && !cli.silent {
                    eprintln!(
                        "decoded {} payload(s) for trigger '{rule_name}'",
                        payloads.len()
                    );
                    for (index, payload) in payloads.iter().enumerate() {
                        eprintln!("  [{index}] {payload:?}");
                    }
                }

                if let Some(ref socket) = ipc_socket {
                    let vars =
                        route_capture_trigger_via_ipc(socket, &rule_name, &payloads)?;
                    if let Some(export_fmt) = cli.export {
                        let map = visioflow_core::vars_from_resolved(&vars);
                        write_export_output(&map, export_fmt, cli.silent)?;
                    } else if matches!(output_format, RuleOutputFormat::Json) {
                        write_resolved_output(&vars, RuleOutputFormat::Json, cli.silent)?;
                    } else {
                        write_resolved_output(&vars, RuleOutputFormat::Plain, cli.silent)?;
                    }
                } else {
                    let store = open_store(rule_store);
                    let routed = route_capture_trigger(&store, &rule_name, &payloads)
                        .map_err(map_rule_error)?;

                    if let Some(export_fmt) = cli.export {
                        let vars = visioflow_core::vars_from_resolved(&routed.vars);
                        write_export_output(&vars, export_fmt, cli.silent)?;
                    } else if matches!(output_format, RuleOutputFormat::Json) {
                        write_resolved_output(&routed.vars, RuleOutputFormat::Json, cli.silent)?;
                    } else {
                        write_resolved_output(&routed.vars, RuleOutputFormat::Plain, cli.silent)?;
                    }

                    spawn_rule_exec(&routed.rule, &routed.vars).map_err(map_rule_error)?;
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
                write_capture_output(&payloads, action, cli.silent)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

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
            Commands::Rule { .. } | Commands::Daemon { .. } => {}
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
            Commands::Rule { .. } | Commands::Daemon { .. } => panic!("expected capture"),
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
            Commands::Rule { .. } | Commands::Daemon { .. } => {}
        }
    }
}
