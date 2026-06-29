use std::sync::Arc;
use std::time::{Duration, Instant};

use minifb::{Key, Window, WindowOptions};
use visioflow_core::error::{Result, VisioFlowError};
use visioflow_core::opencv_webcam::bracket::{BracketAction, BracketConfig, BracketState};
use visioflow_core::opencv_webcam::exposure_hal::OpenCvExposureHal;
use visioflow_core::opencv_webcam::exposure_probe::{bgr_mean_luma, probe_override_safe};
use visioflow_core::opencv_webcam::exposure_table::VideoBackend;
use visioflow_core::opencv_webcam::frame_stream::{FrameStream, OpenCvCaptureDriver};
use visioflow_core::opencv_webcam::models::resolve_model_paths;
use visioflow_core::opencv_webcam::wechat_decoder::WeChatCnnDecoder;
use visioflow_core::traits::{BgrFrame, CnnQrDecoder, ExposureHal, LiveFrameSource, OpticalFilterKind};

use crate::commands::capture::{ExposureBracketMode, PreviewPosition};
use crate::decode_worker::{AsyncDecodeWorker, DecodeOutcome};
use crate::preview_overlay::draw_preview_status_overlay;
use crate::screen_bounds::{apply_anchored_preview_position, primary_work_area};
use crate::webcam_preview::{
    downscale_rgb_to_minifb_buffer, preview_dimensions_from_screen, should_attempt_decode,
};

/// Default seconds to scan the webcam before timing out.
pub const DEFAULT_WEBCAM_TIMEOUT_SECS: u64 = 20;

/// Milliseconds to wait at each exposure before advancing the bracket.
pub const DEFAULT_EXPOSURE_STEP_MS: u64 = 100;

/// Frames to discard after each exposure change so the sensor can settle.
pub const DEFAULT_EXPOSURE_FLUSH_GRABS: u32 = 2;

/// Tunable timing for webcam decode and exposure bracket cycling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebcamTiming {
    pub exposure_step: Duration,
    pub flush_grabs: u32,
    pub decode_interval: Duration,
}

impl WebcamTiming {
    #[must_use]
    pub fn from_ms(exposure_step_ms: u64, flush_grabs: u32, decode_interval_ms: u64) -> Self {
        Self {
            exposure_step: Duration::from_millis(exposure_step_ms.clamp(50, 2_000)),
            flush_grabs: flush_grabs.clamp(1, 30),
            decode_interval: crate::webcam_preview::decode_interval_from_ms(decode_interval_ms),
        }
    }

    #[must_use]
    pub fn defaults() -> Self {
        Self::from_ms(
            DEFAULT_EXPOSURE_STEP_MS,
            DEFAULT_EXPOSURE_FLUSH_GRABS,
            crate::webcam_preview::DEFAULT_DECODE_INTERVAL_MS,
        )
    }

    #[must_use]
    pub fn bracket_config(&self) -> BracketConfig {
        BracketConfig {
            primary_timeout: self.exposure_step,
            flush_grabs: self.flush_grabs,
        }
    }
}

/// Open a live preview window, scan frames for up to `timeout_secs`, and decode the first QR found.
pub fn capture_webcam_with_preview(
    _filter: OpticalFilterKind,
    timeout_secs: u64,
    verbose: bool,
    preview_position: PreviewPosition,
    preview_scale: f32,
    timing: WebcamTiming,
    exposure_bracket: ExposureBracketMode,
) -> Result<Vec<String>> {
    let model_paths = resolve_model_paths()?;
    let decoder = Arc::new(WeChatCnnDecoder::init(&model_paths)?);
    let stream = Arc::new(FrameStream::start(OpenCvCaptureDriver::open_default()?));
    let backend = if cfg!(target_os = "windows") {
        VideoBackend::Dshow
    } else if cfg!(target_os = "linux") {
        VideoBackend::V4l2
    } else {
        VideoBackend::Other
    };
    let exposure = OpenCvExposureHal::new(Arc::clone(&stream), backend);
    exposure.enable_auto_exposure()?;
    if verbose {
        eprintln!("decode: using WeChat CNN scanner with temporal exposure bracketing");
        eprintln!(
            "timing: exposure step {} ms, flush {} grabs, decode every {} ms",
            timing.exposure_step.as_millis(),
            timing.flush_grabs,
            timing.decode_interval.as_millis(),
        );
    }
    scan_with_preview(
        stream,
        decoder,
        &exposure,
        timeout_secs,
        verbose,
        preview_position,
        preview_scale,
        timing,
        exposure_bracket,
    )
}

fn resolve_bracketing_enabled<F>(
    mode: ExposureBracketMode,
    frame_source: &F,
    exposure: &OpenCvExposureHal<OpenCvCaptureDriver>,
    flush_grabs: u32,
    verbose: bool,
) -> Result<bool>
where
    F: LiveFrameSource,
{
    match mode {
        ExposureBracketMode::On => Ok(true),
        ExposureBracketMode::Off => Ok(false),
        ExposureBracketMode::Auto => {
            let safe = probe_override_safe(frame_source, exposure, flush_grabs)?;
            if verbose {
                if safe {
                    eprintln!("exposure: bracketing enabled after probe");
                } else {
                    eprintln!(
                        "exposure: bracketing disabled (camera unsafe for manual override)"
                    );
                }
            }
            Ok(safe)
        }
    }
}

fn scan_with_preview<F, D>(
    frame_source: F,
    decoder: Arc<D>,
    exposure: &OpenCvExposureHal<OpenCvCaptureDriver>,
    timeout_secs: u64,
    verbose: bool,
    preview_position: PreviewPosition,
    preview_scale: f32,
    timing: WebcamTiming,
    exposure_bracket: ExposureBracketMode,
) -> Result<Vec<String>>
where
    F: LiveFrameSource,
    D: CnnQrDecoder + 'static,
{
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let first = frame_source.latest_frame()?;
    let capture_width = first.width;
    let capture_height = first.height;

    let work_area = primary_work_area().unwrap_or_else(|| {
        let w = capture_width.max(1);
        let h = capture_height.max(1);
        crate::screen_bounds::ScreenBounds {
            x: 0,
            y: 0,
            width: w,
            height: h,
        }
    });
    let (preview_width, preview_height) = preview_dimensions_from_screen(
        capture_width,
        capture_height,
        work_area.width,
        work_area.height,
        preview_scale,
    );

    let bracket_config = timing.bracket_config();
    let mut bracketing_enabled = resolve_bracketing_enabled(
        exposure_bracket,
        &frame_source,
        exposure,
        timing.flush_grabs,
        verbose,
    )?;
    let decode_worker = AsyncDecodeWorker::spawn(decoder);

    let mut window = Window::new(
        &format!(
            "VisioFlow Webcam ({capture_width}x{capture_height}) — OpenCV WeChat Scanner"
        ),
        preview_width as usize,
        preview_height as usize,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| VisioFlowError::Capture(format!("failed to open preview window: {e}")))?;
    apply_anchored_preview_position(
        &mut window,
        &work_area,
        preview_position,
        preview_width,
        preview_height,
    );

    let mut window_buffer =
        Vec::with_capacity((preview_width as usize) * (preview_height as usize));

    let mut last_decode = Instant::now();
    let mut bracket_state = BracketState::new(
        bracket_config,
        Instant::now(),
        exposure.step_count(),
    );
    let mut in_override_mode = false;
    let mut decode_in_flight = false;

    while window.is_open() && !window.is_key_down(Key::Escape) && Instant::now() < deadline {
        let frame = frame_source.latest_frame()?;
        show_frame_in_window(
            &frame,
            preview_width,
            preview_height,
            bracketing_enabled,
            &mut window,
            &mut window_buffer,
        )?;

        while let Some(outcome) = decode_worker.try_recv() {
            decode_in_flight = false;
            match outcome {
                DecodeOutcome::Success(payloads) => {
                    if verbose {
                        eprintln!("decode: wechat hit");
                    }
                    return Ok(payloads);
                }
                DecodeOutcome::NoPayloads if bracketing_enabled => {
                    if let Some(disabled) = handle_decode_miss(
                        &mut bracket_state,
                        bracket_config,
                        exposure,
                        &frame_source,
                        &decode_worker,
                        &mut in_override_mode,
                        &mut bracketing_enabled,
                        verbose,
                    )? {
                        if disabled && verbose {
                            eprintln!(
                                "exposure: bracketing disabled mid-scan (runtime plunge detected)"
                            );
                        }
                    }
                }
                DecodeOutcome::NoPayloads => {}
                DecodeOutcome::Failed(error) => return Err(error),
            }
        }

        if !decode_in_flight && should_attempt_decode(last_decode.elapsed(), timing.decode_interval)
        {
            if decode_worker.try_submit(frame) {
                decode_in_flight = true;
                last_decode = Instant::now();
            }
        }

        window.update();
    }

    if Instant::now() >= deadline {
        return Err(VisioFlowError::Capture(format!(
            "webcam capture timed out after {timeout_secs} seconds"
        )));
    }

    Err(VisioFlowError::Capture(
        "webcam preview closed before a QR code was detected".into(),
    ))
}

/// Returns `Some(true)` when bracketing was disabled due to a runtime plunge.
fn handle_decode_miss<F>(
    bracket_state: &mut BracketState,
    bracket_config: BracketConfig,
    exposure: &OpenCvExposureHal<OpenCvCaptureDriver>,
    frame_source: &F,
    decode_worker: &AsyncDecodeWorker,
    in_override_mode: &mut bool,
    bracketing_enabled: &mut bool,
    verbose: bool,
) -> Result<Option<bool>>
where
    F: LiveFrameSource,
{
    match bracket_state.on_primary_decode_failure(Instant::now()) {
        BracketAction::KeepPrimary => Ok(None),
        BracketAction::AdvanceExposureStep {
            step_index,
            flush_grabs,
        } => {
            let luma_before = bgr_mean_luma(&frame_source.latest_frame()?);
            if !*in_override_mode {
                *in_override_mode = true;
            }
            let max_step = exposure.step_count().saturating_sub(1);
            let sparse_step = (step_index.saturating_mul(2)).min(max_step);
            exposure.set_step(sparse_step)?;
            frame_source.flush_after_exposure_change(flush_grabs)?;
            decode_worker.drain_pending_outcomes();

            let luma_after = bgr_mean_luma(&frame_source.latest_frame()?);
            if luma_before >= 8.0 && luma_after < luma_before * 0.60 {
                exposure.enable_auto_exposure()?;
                frame_source.flush_after_exposure_change(flush_grabs)?;
                *in_override_mode = false;
                *bracketing_enabled = false;
                *bracket_state = BracketState::new(
                    bracket_config,
                    Instant::now(),
                    exposure.step_count(),
                );
                return Ok(Some(true));
            }

            if verbose {
                eprintln!(
                    "exposure: sparse bracket step {sparse_step}, flushed {flush_grabs} grabs"
                );
            }
            if sparse_step >= max_step {
                exposure.enable_auto_exposure()?;
                *in_override_mode = false;
                *bracket_state = BracketState::new(
                    bracket_config,
                    Instant::now(),
                    exposure.step_count(),
                );
                if verbose {
                    eprintln!("exposure: returning to auto/no-override mode");
                }
            }
            Ok(None)
        }
        BracketAction::Exhausted => {
            exposure.enable_auto_exposure()?;
            *in_override_mode = false;
            *bracket_state = BracketState::new(
                bracket_config,
                Instant::now(),
                exposure.step_count(),
            );
            if verbose {
                eprintln!("exposure: bracket exhausted, restarting in auto mode");
            }
            Ok(None)
        }
    }
}

fn show_frame_in_window(
    frame: &BgrFrame,
    preview_width: u32,
    preview_height: u32,
    bracketing_enabled: bool,
    window: &mut Window,
    buffer: &mut Vec<u32>,
) -> Result<()> {
    let rgb = bgr_to_rgb(frame);
    downscale_rgb_to_minifb_buffer(
        &rgb,
        frame.width,
        frame.height,
        preview_width,
        preview_height,
        buffer,
    );
    draw_preview_status_overlay(buffer, preview_width, preview_height, bracketing_enabled);
    window
        .update_with_buffer(buffer, preview_width as usize, preview_height as usize)
        .map_err(|e| VisioFlowError::Capture(format!("failed to update preview window: {e}")))?;
    Ok(())
}

fn bgr_to_rgb(frame: &BgrFrame) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(frame.data.len());
    for chunk in frame.data.chunks_exact(3) {
        rgb.push(chunk[2]);
        rgb.push(chunk[1]);
        rgb.push(chunk[0]);
    }
    rgb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_webcam_timeout_is_twenty_seconds() {
        assert_eq!(DEFAULT_WEBCAM_TIMEOUT_SECS, 20);
    }

    #[test]
    fn webcam_timing_defaults_match_fast_profile() {
        let timing = WebcamTiming::defaults();
        assert_eq!(timing.exposure_step, Duration::from_millis(100));
        assert_eq!(timing.flush_grabs, 2);
        assert_eq!(timing.decode_interval, Duration::from_millis(100));
        let bracket = timing.bracket_config();
        assert_eq!(bracket.primary_timeout, Duration::from_millis(100));
        assert_eq!(bracket.flush_grabs, 2);
    }

    #[test]
    fn webcam_timing_clamps_out_of_range_values() {
        let timing = WebcamTiming::from_ms(10, 0, 5_000);
        assert_eq!(timing.exposure_step, Duration::from_millis(50));
        assert_eq!(timing.flush_grabs, 1);
        assert_eq!(timing.decode_interval, Duration::from_millis(2_000));
    }

    #[test]
    fn bgr_to_rgb_swaps_channels() {
        let frame = BgrFrame::new(1, 1, vec![1, 2, 3]);
        assert_eq!(bgr_to_rgb(&frame), vec![3, 2, 1]);
    }
}
