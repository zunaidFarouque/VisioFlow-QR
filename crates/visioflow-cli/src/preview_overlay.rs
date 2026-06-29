//! Semi-transparent status text drawn over the minifb preview buffer.

use font8x8::UnicodeFonts;
use font8x8::BASIC_FONTS;

pub const STATUS_LINE_PRIMARY: &str = "Scanning QR code...";
pub const STATUS_LINE_SECONDARY: &str = "Brightness changes are normal";
pub const STATUS_LINE_AUTO_ONLY: &str = "Auto exposure only";

const PANEL_ALPHA: f32 = 0.55;
const TEXT_ALPHA: f32 = 0.92;
const PANEL_PADDING: u32 = 6;
const LINE_GAP: u32 = 4;

#[must_use]
pub fn blend_pixel(bg: u32, fg_r: u8, fg_g: u8, fg_b: u8, alpha: f32) -> u32 {
    let alpha = alpha.clamp(0.0, 1.0);
    let inv = 1.0 - alpha;
    let bg_r = ((bg >> 16) & 0xFF) as f32;
    let bg_g = ((bg >> 8) & 0xFF) as f32;
    let bg_b = (bg & 0xFF) as f32;
    let r = (bg_r * inv + f32::from(fg_r) * alpha).round() as u32;
    let g = (bg_g * inv + f32::from(fg_g) * alpha).round() as u32;
    let b = (bg_b * inv + f32::from(fg_b) * alpha).round() as u32;
    (r << 16) | (g << 8) | b
}

#[must_use]
pub fn overlay_scale_for_height(height: u32) -> u32 {
    if height >= 240 { 2 } else { 1 }
}

/// Draw a reassurance banner along the bottom of the preview buffer.
pub fn draw_preview_status_overlay(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    bracketing_enabled: bool,
) {
    if width == 0 || height == 0 || buffer.len() < (width * height) as usize {
        return;
    }

    let secondary = if bracketing_enabled {
        STATUS_LINE_SECONDARY
    } else {
        STATUS_LINE_AUTO_ONLY
    };

    let scale = overlay_scale_for_height(height);
    let char_h = 8 * scale;
    let lines = [STATUS_LINE_PRIMARY, secondary];
    let text_block_h = char_h * lines.len() as u32
        + LINE_GAP * lines.len().saturating_sub(1) as u32
        + PANEL_PADDING * 2;
    let panel_top = height.saturating_sub(text_block_h);

    fill_rect_alpha(
        buffer,
        width,
        height,
        0,
        panel_top,
        width,
        height,
        0,
        0,
        0,
        PANEL_ALPHA,
    );

    let mut y = panel_top + PANEL_PADDING;
    for line in lines {
        draw_text_alpha(buffer, width, height, PANEL_PADDING, y, line, scale, 255, 255, 255, TEXT_ALPHA);
        y += char_h + LINE_GAP;
    }
}

fn fill_rect_alpha(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    r: u8,
    g: u8,
    b: u8,
    alpha: f32,
) {
    let x_end = x1.min(width);
    let y_end = y1.min(height);
    for y in y0.min(height)..y_end {
        let row = (y * width) as usize;
        for x in x0.min(width)..x_end {
            let idx = row + x as usize;
            buffer[idx] = blend_pixel(buffer[idx], r, g, b, alpha);
        }
    }
}

fn draw_text_alpha(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    text: &str,
    scale: u32,
    r: u8,
    g: u8,
    b: u8,
    alpha: f32,
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        draw_char_alpha(
            buffer,
            width,
            height,
            cursor_x,
            y,
            ch,
            scale,
            r,
            g,
            b,
            alpha,
        );
        cursor_x = cursor_x.saturating_add(8 * scale);
    }
}

fn draw_char_alpha(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    ch: char,
    scale: u32,
    r: u8,
    g: u8,
    b: u8,
    alpha: f32,
) {
    let glyph = char_glyph(ch);
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..8 {
            if bits & (1 << col) == 0 {
                continue;
            }
            for sy in 0..scale {
                for sx in 0..scale {
                    let px = x + (col as u32 * scale) + sx;
                    let py = y + (row as u32 * scale) + sy;
                    if px >= width || py >= height {
                        continue;
                    }
                    let idx = (py * width + px) as usize;
                    buffer[idx] = blend_pixel(buffer[idx], r, g, b, alpha);
                }
            }
        }
    }
}

fn char_glyph(ch: char) -> [u8; 8] {
    BASIC_FONTS
        .get(ch)
        .or_else(|| BASIC_FONTS.get('?'))
        .unwrap_or([0; 8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_pixel_halves_white_to_gray() {
        assert_eq!(blend_pixel(0x00_FF_FF_FF, 0, 0, 0, 0.5), 0x00_80_80_80);
    }

    #[test]
    fn overlay_leaves_top_row_untouched() {
        let width = 80;
        let height = 40;
        let mut buffer = vec![0x00_FF_00_00; (width * height) as usize];
        draw_preview_status_overlay(&mut buffer, width, height, true);
        assert_eq!(buffer[0], 0x00_FF_00_00);
    }

    #[test]
    fn overlay_darkens_bottom_band() {
        let width = 80;
        let height = 40;
        let mut buffer = vec![0x00_FF_FF_FF; (width * height) as usize];
        draw_preview_status_overlay(&mut buffer, width, height, true);
        let bottom_left = buffer[((height - 1) * width) as usize];
        assert_ne!(bottom_left, 0x00_FF_FF_FF);
    }

    #[test]
    fn overlay_scale_uses_two_on_tall_previews() {
        assert_eq!(overlay_scale_for_height(239), 1);
        assert_eq!(overlay_scale_for_height(240), 2);
    }
}
