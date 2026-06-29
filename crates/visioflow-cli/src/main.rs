use clap::{Parser, Subcommand, ValueEnum};
use visioflow_cli::commands::capture::{
    run_capture, write_capture_output, CaptureAction, CaptureArgs, CaptureFilter, CaptureSource,
    ExposureBracketMode, PreviewPosition,
};
use visioflow_cli::webcam_preview::DEFAULT_DECODE_INTERVAL_MS;
use visioflow_cli::webcam_session::{
    DEFAULT_EXPOSURE_FLUSH_GRABS, DEFAULT_EXPOSURE_STEP_MS,
};

#[derive(Debug, Parser)]
#[command(name = "visioflow", about = "Optical automation engine for visual payload routing")]
struct Cli {
    #[arg(long, value_enum, default_value = "plain")]
    output: OutputFormat,

    #[arg(long, short, global = true)]
    verbose: bool,

    #[arg(long, global = true)]
    silent: bool,

    #[arg(long, value_enum)]
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
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> visioflow_core::error::Result<()> {
    let cli = Cli::parse();

    if cli.verbose && !cli.silent {
        eprintln!("visioflow starting");
    }

    if cli.export.is_some() {
        return Err(visioflow_core::VisioFlowError::UnsupportedAction(
            "--export is not implemented yet".into(),
        ));
    }

    if cli.ipc_socket.is_some() {
        return Err(visioflow_core::VisioFlowError::UnsupportedAction(
            "--ipc-socket is not implemented yet".into(),
        ));
    }

    match cli.command {
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
            })?;

            if matches!(cli.output, OutputFormat::Json) {
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
        }
    }
}
