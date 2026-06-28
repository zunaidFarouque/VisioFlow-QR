//! Helpers for scaling webcam frames into a small preview window buffer.

use std::time::Duration;

/// Default maximum width of the live preview window in pixels (display only).
pub const DEFAULT_PREVIEW_MAX_WIDTH: u32 = 640;

/// Target webcam capture resolution — full sensor quality for QR decode.
pub const DEFAULT_WEBCAM_RESOLUTION: (u32, u32) = (1920, 1080);

/// Minimum interval between QR decode attempts on the capture thread.
pub const DEFAULT_DECODE_INTERVAL: Duration = Duration::from_millis(350);

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
        assert!(should_attempt_decode(
            Duration::from_millis(350),
            Duration::from_millis(350)
        ));
        assert!(!should_attempt_decode(
            Duration::from_millis(50),
            Duration::from_millis(350)
        ));
    }
}
