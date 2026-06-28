use clap::{Parser, Subcommand, ValueEnum};
use visioflow_cli::commands::capture::{
    run_capture, write_capture_output, CaptureAction, CaptureArgs, CaptureFilter, CaptureSource,
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
        } => {
            let payloads = run_capture(CaptureArgs {
                source,
                filter,
                action,
                input_image,
                timeout_secs: timeout,
                verbose: cli.verbose,
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
