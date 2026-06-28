use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use image::{DynamicImage, ImageBuffer, Rgb};
use minifb::{Key, Window, WindowOptions};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{ApiBackend, CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;
use visioflow_core::capture::decode_captured_frame_live_with_profile;
use visioflow_core::decode::{alternating_live_decode_profile, LiveDecodeProfile, RqrrDecoder};
use visioflow_core::error::{Result, VisioFlowError};
use visioflow_core::traits::OpticalFilterKind;

use crate::manual_exposure::adjust_manual_ev_on_arrow_keys;
use crate::webcam_exposure::{
    log_exposure_status, WebcamExposureController, EXPOSURE_SETTLE_FRAMES,
};
use crate::webcam_preview::{
    downscale_rgb_to_minifb_buffer, preview_dimensions, should_attempt_decode,
    DEFAULT_DECODE_INTERVAL, DEFAULT_PREVIEW_MAX_WIDTH, DEFAULT_WEBCAM_RESOLUTION,
};

/// Default seconds to scan the webcam before timing out.
pub const DEFAULT_WEBCAM_TIMEOUT_SECS: u64 = 20;

type SharedFrame = Arc<ImageBuffer<Rgb<u8>, Vec<u8>>>;

struct DecodeJobRequest {
    frame: SharedFrame,
    filter: OpticalFilterKind,
    profile: LiveDecodeProfile,
}

/// Open a live preview window, scan frames for up to `timeout_secs`, and decode the first QR found.
pub fn capture_webcam_with_preview(
    filter: OpticalFilterKind,
    timeout_secs: u64,
    verbose: bool,
) -> Result<Vec<String>> {
    let mut camera = open_webcam_camera()?;

    camera.open_stream().map_err(|e| {
        VisioFlowError::Capture(format!("failed to start webcam stream: {e}"))
    })?;

    for _ in 0..3 {
        let _ = camera.frame();
    }

    let mut exposure = WebcamExposureController::probe(&camera, verbose);
    log_exposure_status(&exposure, true);
    if !exposure.is_supported() {
        eprintln!(
            "exposure: this camera may not allow exposure adjustment — try another webcam or driver"
        );
    } else if verbose {
        eprintln!(
            "exposure: mode={} — focus preview, use ↑/↓ for ±0.5 EV adjustment",
            exposure.control_kind_label()
        );
        eprintln!("exposure: return to 0 EV to restore auto brightness");
    }

    let result = scan_with_preview(
        &mut camera,
        filter,
        timeout_secs,
        verbose,
        &mut exposure,
    );

    exposure.restore(&mut camera);
    camera.stop_stream().ok();
    result
}

/// Prefer uncompressed formats (no per-frame JPEG decode) at 1080p, then fall back.
fn open_webcam_camera() -> Result<Camera> {
    let index = CameraIndex::Index(0);
    let (width, height) = DEFAULT_WEBCAM_RESOLUTION;

    let format_candidates = [
        FrameFormat::NV12,
        FrameFormat::YUYV,
        FrameFormat::RAWRGB,
        FrameFormat::MJPEG,
    ];

    for frame_format in format_candidates {
        let camera_format = CameraFormat::new_from(width, height, frame_format, 30);
        let requested =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(camera_format));

        if let Ok(camera) = Camera::with_backend(index.clone(), requested, ApiBackend::MediaFoundation) {
            return Ok(camera);
        }
    }

    let fallback = CameraFormat::new_from(width, height, FrameFormat::MJPEG, 30);
    let requested =
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(fallback));

    Camera::with_backend(index, requested, ApiBackend::MediaFoundation).or_else(|_| {
        let requested =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(fallback));
        Camera::new(CameraIndex::Index(0), requested)
    })
    .map_err(|e| {
        VisioFlowError::Capture(format!("failed to open webcam at {width}x{height}: {e}"))
    })
}

fn scan_with_preview(
    camera: &mut Camera,
    filter: OpticalFilterKind,
    timeout_secs: u64,
    verbose: bool,
    hw_exposure: &mut WebcamExposureController,
) -> Result<Vec<String>> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let (result_tx, result_rx) = mpsc::channel::<Vec<String>>();
    let decode_busy = Arc::new(AtomicBool::new(false));

    let first_rgb = read_camera_rgb(camera).ok_or_else(|| {
        VisioFlowError::Capture("failed to read initial webcam frame".into())
    })?;
    let capture_width = first_rgb.width();
    let capture_height = first_rgb.height();

    let (preview_width, preview_height) =
        preview_dimensions(capture_width, capture_height, DEFAULT_PREVIEW_MAX_WIDTH);

    let mut window = Window::new(
        &format!(
            "VisioFlow Webcam ({capture_width}x{capture_height}) — ↑↓ sensor exposure"
        ),
        preview_width as usize,
        preview_height as usize,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| VisioFlowError::Capture(format!("failed to open preview window: {e}")))?;

    let mut window_buffer =
        Vec::with_capacity((preview_width as usize) * (preview_height as usize));

    let mut manual_ev = 0.0_f32;
    let mut keys_held = HashSet::new();
    let mut last_decode = Instant::now();
    let mut decode_attempt: u64 = 0;
    let mut settling_until_frame: u64 = 0;
    let mut frame_index: u64 = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) && Instant::now() < deadline {
        if let Ok(payloads) = result_rx.try_recv() {
            return Ok(payloads);
        }

        frame_index += 1;

        if adjust_manual_ev_on_arrow_keys(&window, &mut manual_ev, &mut keys_held) {
            eprintln!("exposure: {manual_ev:+.1} EV (↑ brighter, ↓ darker) — sensor adjusting…");
            hw_exposure.apply_manual_ev(camera, manual_ev, verbose)?;
            settling_until_frame = frame_index + u64::from(EXPOSURE_SETTLE_FRAMES);
            drain_camera_frames(camera, EXPOSURE_SETTLE_FRAMES);
        }

        let Some(rgb) = read_camera_rgb(camera) else {
            window.update();
            continue;
        };

        let shared = Arc::new(rgb);
        show_frame_in_window(
            &shared,
            preview_width,
            preview_height,
            &mut window,
            &mut window_buffer,
        )?;

        let exposure_settled = frame_index >= settling_until_frame;

        if exposure_settled
            && should_attempt_decode(last_decode.elapsed(), DEFAULT_DECODE_INTERVAL)
            && !decode_busy.load(Ordering::Relaxed)
        {
            last_decode = Instant::now();
            decode_attempt += 1;
            let profile = alternating_live_decode_profile(decode_attempt);
            spawn_decode_job(
                DecodeJobRequest {
                    frame: shared,
                    filter,
                    profile,
                },
                Arc::clone(&decode_busy),
                &result_tx,
            );
        }

        window.update();
    }

    if let Ok(payloads) = result_rx.try_recv() {
        return Ok(payloads);
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

fn drain_camera_frames(camera: &mut Camera, count: u32) {
    for _ in 0..count {
        let _ = camera.frame();
    }
}

fn show_frame_in_window(
    frame: &SharedFrame,
    preview_width: u32,
    preview_height: u32,
    window: &mut Window,
    buffer: &mut Vec<u32>,
) -> Result<()> {
    downscale_rgb_to_minifb_buffer(
        frame.as_raw(),
        frame.width(),
        frame.height(),
        preview_width,
        preview_height,
        buffer,
    );
    window
        .update_with_buffer(buffer, preview_width as usize, preview_height as usize)
        .map_err(|e| VisioFlowError::Capture(format!("failed to update preview window: {e}")))?;
    Ok(())
}

fn spawn_decode_job(
    request: DecodeJobRequest,
    decode_busy: Arc<AtomicBool>,
    result_tx: &mpsc::Sender<Vec<String>>,
) {
    if decode_busy
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return;
    }

    let result_tx = result_tx.clone();
    thread::spawn(move || {
        let _guard = DecodeBusyGuard(decode_busy);
        if let Ok(Some(payloads)) = try_decode(&request) {
            let _ = result_tx.send(payloads);
        }
    });
}

struct DecodeBusyGuard(Arc<AtomicBool>);

impl Drop for DecodeBusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn read_camera_rgb(camera: &mut Camera) -> Option<ImageBuffer<Rgb<u8>, Vec<u8>>> {
    camera
        .frame()
        .ok()?
        .decode_image::<RgbFormat>()
        .ok()
}

fn try_decode(request: &DecodeJobRequest) -> Result<Option<Vec<String>>> {
    let decoder = RqrrDecoder;
    let frame = DynamicImage::ImageRgb8(request.frame.as_ref().clone());
    match decode_captured_frame_live_with_profile(
        &frame,
        request.filter,
        &decoder,
        request.profile,
    ) {
        Ok(payloads) => Ok(Some(payloads)),
        Err(VisioFlowError::NoPayloads) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_webcam_timeout_is_twenty_seconds() {
        assert_eq!(DEFAULT_WEBCAM_TIMEOUT_SECS, 20);
    }
}
