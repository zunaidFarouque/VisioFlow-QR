use clap::ValueEnum;
use visioflow_core::capture::CaptureEngine;
use visioflow_core::decode::RqrrDecoder;
use visioflow_core::error::Result;
use visioflow_core::traits::{FrameSource, OpticalFilterKind};

use crate::capture::{FileFrameSource, SnipFrameSource};
use crate::webcam_session::{capture_webcam_with_preview, WebcamTiming, DEFAULT_WEBCAM_TIMEOUT_SECS};

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
        CaptureSource::Webcam => capture_webcam_with_preview(
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
        ),
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
