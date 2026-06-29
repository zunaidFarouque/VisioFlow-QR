//! Helpers for scaling webcam frames into a small preview window buffer.

use std::time::Duration;

/// Default maximum width of the live preview window in pixels (display only).
pub const DEFAULT_PREVIEW_MAX_WIDTH: u32 = 640;
pub const DEFAULT_PREVIEW_SCALE: f32 = 0.12;
pub const MIN_PREVIEW_HEIGHT: u32 = 200;

/// Target webcam capture resolution — full sensor quality for QR decode.
pub const DEFAULT_WEBCAM_RESOLUTION: (u32, u32) = (1920, 1080);

/// Minimum interval between QR decode attempts on the capture thread.
pub const DEFAULT_DECODE_INTERVAL_MS: u64 = 100;
pub const DEFAULT_DECODE_INTERVAL: Duration = Duration::from_millis(DEFAULT_DECODE_INTERVAL_MS);

/// Clamp decode interval to a sane range for live webcam scanning.
#[must_use]
pub fn decode_interval_from_ms(ms: u64) -> Duration {
    Duration::from_millis(ms.clamp(50, 2_000))
}

/// Returns true when a decode attempt is due based on the elapsed time since the last one.
pub fn should_attempt_decode(elapsed: Duration, interval: Duration) -> bool {
    elapsed >= interval
}

/// Compute preview dimensions that fit within `max_width` while preserving aspect ratio.
pub fn preview_dimensions(width: u32, height: u32, max_width: u32) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (1, 1);
    }
    if width <= max_width {
        return (width, height);
    }

    let scale = max_width as f64 / width as f64;
    let preview_width = max_width;
    let preview_height = (height as f64 * scale).round().max(1.0) as u32;
    (preview_width, preview_height)
}

#[must_use]
pub fn clamp_preview_scale(scale: f32) -> f32 {
    if !scale.is_finite() {
        return DEFAULT_PREVIEW_SCALE;
    }
    scale.clamp(0.05, 1.0)
}

/// Compute preview dimensions from screen size and capture aspect ratio.
#[must_use]
pub fn preview_dimensions_from_screen(
    capture_width: u32,
    capture_height: u32,
    screen_width: u32,
    screen_height: u32,
    scale: f32,
) -> (u32, u32) {
    if capture_width == 0 || capture_height == 0 || screen_width == 0 || screen_height == 0 {
        return (1, 1);
    }

    let clamped = clamp_preview_scale(scale);
    let target_height = ((screen_height as f32 * clamped).round() as u32).max(MIN_PREVIEW_HEIGHT);
    let mut preview_height = target_height.min(screen_height);
    if preview_height == 0 {
        preview_height = 1;
    }

    let mut preview_width =
        ((preview_height as f64 * capture_width as f64) / capture_height as f64).round() as u32;
    preview_width = preview_width.max(1).min(screen_width);

    if preview_width == screen_width {
        preview_height = ((preview_width as f64 * capture_height as f64) / capture_width as f64)
            .round() as u32;
        preview_height = preview_height.max(1).min(screen_height);
    }

    (preview_width, preview_height)
}

/// Single-pass nearest-neighbor downscale from RGB8 source straight into a minifb buffer.
pub fn downscale_rgb_to_minifb_buffer(
    src: &[u8],
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
    out: &mut Vec<u32>,
) {
    let pixel_count = (dst_width as usize) * (dst_height as usize);
    out.clear();
    out.reserve(pixel_count);

    if src_width == 0 || src_height == 0 || dst_width == 0 || dst_height == 0 {
        out.resize(pixel_count, 0);
        return;
    }

    if src_width == dst_width && src_height == dst_height {
        rgb_to_preview_buffer_into(src, dst_width, dst_height, out);
        return;
    }

    let x_ratio = src_width as f32 / dst_width as f32;
    let y_ratio = src_height as f32 / dst_height as f32;
    let src_stride = (src_width * 3) as usize;

    for dy in 0..dst_height {
        let sy = ((dy as f32 * y_ratio) as u32).min(src_height - 1);
        let row_base = (sy as usize) * src_stride;
        for dx in 0..dst_width {
            let sx = ((dx as f32 * x_ratio) as u32).min(src_width - 1);
            let i = row_base + (sx as usize * 3);
            if i + 2 < src.len() {
                out.push(
                    (u32::from(src[i]) << 16)
                        | (u32::from(src[i + 1]) << 8)
                        | u32::from(src[i + 2]),
                );
            } else {
                out.push(0);
            }
        }
    }
}

/// Convert RGB8 bytes into a minifb ARGB buffer (`0x00RRGGBB`), reusing `out` when possible.
pub fn rgb_to_preview_buffer_into(rgb: &[u8], width: u32, height: u32, out: &mut Vec<u32>) {
    let pixel_count = (width as usize) * (height as usize);
    out.clear();
    out.reserve(pixel_count);

    for chunk in rgb.chunks_exact(3).take(pixel_count) {
        out.push(
            (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]),
        );
    }

    out.resize(pixel_count, 0);
}

/// Convert RGB8 bytes into a minifb ARGB buffer (`0x00RRGGBB`).
pub fn rgb_to_preview_buffer(rgb: &[u8], width: u32, height: u32) -> Vec<u32> {
    let mut buffer = Vec::new();
    rgb_to_preview_buffer_into(rgb, width, height, &mut buffer);
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_dimensions_scales_1080p_to_640_wide() {
        assert_eq!(preview_dimensions(1920, 1080, 640), (640, 360));
    }

    #[test]
    fn preview_dimensions_keeps_small_frames() {
        assert_eq!(preview_dimensions(320, 240, 640), (320, 240));
    }

    #[test]
    fn rgb_to_preview_buffer_packs_pixels() {
        let buffer = rgb_to_preview_buffer(&[255, 128, 0], 1, 1);
        assert_eq!(buffer, vec![0x00_FF_80_00]);
    }

    #[test]
    fn downscale_rgb_to_minifb_buffer_samples_top_left_pixel() {
        let src = vec![255u8, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0];
        let mut out = Vec::new();
        downscale_rgb_to_minifb_buffer(&src, 2, 2, 1, 1, &mut out);
        assert_eq!(out, vec![0x00_FF_00_00]);
    }

    #[test]
    fn should_attempt_decode_after_interval_elapses() {
        let interval = decode_interval_from_ms(DEFAULT_DECODE_INTERVAL_MS);
        assert!(should_attempt_decode(interval, interval));
        assert!(!should_attempt_decode(
            Duration::from_millis(50),
            interval
        ));
    }

    #[test]
    fn preview_dimensions_from_screen_honors_min_height() {
        let (w, h) = preview_dimensions_from_screen(1920, 1080, 1920, 900, 0.1);
        assert!(h >= MIN_PREVIEW_HEIGHT);
        assert!(w > h);
    }

    #[test]
    fn preview_dimensions_from_screen_preserves_aspect() {
        let (w, h) = preview_dimensions_from_screen(1920, 1080, 1920, 1080, 0.25);
        let ratio = w as f64 / h as f64;
        assert!((ratio - (1920.0 / 1080.0)).abs() < 0.02);
    }
}
