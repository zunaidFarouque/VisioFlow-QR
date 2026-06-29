//! Startup probe to detect webcams that plunge dark on manual exposure override.

use crate::error::Result;
use crate::opencv_webcam::exposure_hal::OpenCvExposureHal;
use crate::opencv_webcam::frame_stream::CaptureDriver;
use crate::traits::{BgrFrame, ExposureHal, LiveFrameSource};

/// Relative luma drop above this threshold marks override as unsafe.
const LUMA_DROP_RATIO: f64 = 0.60;

/// Minimum auto-exposure luma required for a meaningful probe.
const MIN_PROBE_LUMA: f64 = 8.0;

const PROBE_SETTLE_GRABS: u32 = 2;

/// Mean BT.601 luma of a BGR frame, subsampling every 8th pixel for speed.
#[must_use]
pub fn bgr_mean_luma(frame: &BgrFrame) -> f64 {
    if frame.width == 0 || frame.height == 0 || frame.data.is_empty() {
        return 0.0;
    }

    let stride = (frame.width as usize).saturating_mul(3);
    let row_step = 8usize;
    let col_step = 8usize;
    let mut sum = 0.0;
    let mut count = 0.0;

    let mut y = 0usize;
    while y < frame.height as usize {
        let row_base = y * stride;
        let mut x = 0usize;
        while x < frame.width as usize {
            let i = row_base + x * 3;
            if i + 2 < frame.data.len() {
                let b = f64::from(frame.data[i]);
                let g = f64::from(frame.data[i + 1]);
                let r = f64::from(frame.data[i + 2]);
                sum += 0.114 * b + 0.587 * g + 0.299 * r;
                count += 1.0;
            }
            x += col_step;
        }
        y += row_step;
    }

    if count == 0.0 {
        0.0
    } else {
        sum / count
    }
}

/// Returns true when sparse exposure bracketing is unlikely to plunge the preview dark.
pub fn probe_override_safe<D: CaptureDriver + 'static>(
    frame_source: &impl LiveFrameSource,
    exposure: &OpenCvExposureHal<D>,
    flush_grabs: u32,
) -> Result<bool> {
    exposure.enable_auto_exposure()?;
    frame_source.flush_after_exposure_change(PROBE_SETTLE_GRABS)?;
    let auto_frame = frame_source.latest_frame()?;
    let luma_auto = bgr_mean_luma(&auto_frame);

    if luma_auto < MIN_PROBE_LUMA {
        exposure.enable_auto_exposure()?;
        return Ok(false);
    }

    let max_step = exposure.step_count().saturating_sub(1);
    let mut steps_to_test = vec![0usize];
    if max_step > 0 {
        steps_to_test.push(max_step);
    }
    for step in steps_to_test {
        if !override_step_is_safe(frame_source, exposure, flush_grabs, luma_auto, step)? {
            exposure.enable_auto_exposure()?;
            frame_source.flush_after_exposure_change(PROBE_SETTLE_GRABS)?;
            return Ok(false);
        }
        exposure.enable_auto_exposure()?;
        frame_source.flush_after_exposure_change(PROBE_SETTLE_GRABS)?;
    }

    Ok(true)
}

fn override_step_is_safe<D: CaptureDriver + 'static>(
    frame_source: &impl LiveFrameSource,
    exposure: &OpenCvExposureHal<D>,
    flush_grabs: u32,
    luma_auto: f64,
    step: usize,
) -> Result<bool> {
    exposure.set_step(step)?;
    frame_source.flush_after_exposure_change(flush_grabs)?;
    let override_frame = frame_source.latest_frame()?;
    let luma_override = bgr_mean_luma(&override_frame);
    Ok(luma_override >= luma_auto * LUMA_DROP_RATIO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgr_mean_luma_reads_white_pixels() {
        let frame = BgrFrame::new(4, 4, vec![255; 4 * 4 * 3]);
        assert!((bgr_mean_luma(&frame) - 255.0).abs() < 1.0);
    }

    #[test]
    fn bgr_mean_luma_reads_black_pixels() {
        let frame = BgrFrame::new(4, 4, vec![0; 4 * 4 * 3]);
        assert!((bgr_mean_luma(&frame)).abs() < f64::EPSILON);
    }

    #[test]
    fn bgr_mean_luma_detects_plunge() {
        let bright = BgrFrame::new(8, 8, vec![200; 8 * 8 * 3]);
        let dark = BgrFrame::new(8, 8, vec![20; 8 * 8 * 3]);
        let bright_luma = bgr_mean_luma(&bright);
        let dark_luma = bgr_mean_luma(&dark);
        assert!(dark_luma < bright_luma * LUMA_DROP_RATIO);
    }
}
