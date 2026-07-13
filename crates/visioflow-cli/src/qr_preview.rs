use image::RgbaImage;
use minifb::{Key, Window, WindowOptions};
use visioflow_core::error::{Result, VisioFlowError};

use crate::commands::capture::PreviewPosition;
use crate::preview_util::{downscale_rgb_to_minifb_buffer, qr_preview_square_side};
use crate::screen_bounds::{apply_anchored_preview_position, primary_work_area};

/// Show a blocking square preview window with the encoded QR image until Escape or close.
pub fn show_qr_preview_window(
    image: &RgbaImage,
    module_dimension: u32,
    preview_position: PreviewPosition,
) -> Result<()> {
    let (src_width, src_height) = image.dimensions();
    let work_area = primary_work_area().unwrap_or(crate::screen_bounds::ScreenBounds {
        x: 0,
        y: 0,
        width: src_width.max(1),
        height: src_height.max(1),
    });

    let side = qr_preview_square_side(work_area.width, work_area.height, module_dimension);

    let rgb = rgba_to_rgb8(image);
    let mut scaled_buffer = Vec::new();
    downscale_rgb_to_minifb_buffer(&rgb, src_width, src_height, side, side, &mut scaled_buffer);

    let mut window = Window::new(
        "VisioFlow QR",
        side as usize,
        side as usize,
        WindowOptions {
            resize: false,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| VisioFlowError::Encode(format!("failed to open QR preview window: {e}")))?;

    apply_anchored_preview_position(&mut window, &work_area, preview_position, side, side);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window
            .update_with_buffer(&scaled_buffer, side as usize, side as usize)
            .map_err(|e| VisioFlowError::Encode(format!("failed to update QR preview: {e}")))?;
    }

    Ok(())
}

fn rgba_to_rgb8(image: &RgbaImage) -> Vec<u8> {
    let (width, height) = image.dimensions();
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for pixel in image.pixels() {
        let [r, g, b, _a] = pixel.0;
        rgb.extend_from_slice(&[r, g, b]);
    }
    rgb
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn rgba_to_rgb8_strips_alpha() {
        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, Rgba([1, 2, 3, 255]));
        assert_eq!(rgba_to_rgb8(&image), vec![1, 2, 3]);
    }
}
