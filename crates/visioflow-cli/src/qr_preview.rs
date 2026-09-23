use image::RgbaImage;
use minifb::{Key, Window, WindowOptions};
use visioflow_core::error::{Result, VisioFlowError};

use crate::commands::capture::PreviewPosition;
use crate::preview_util::{downscale_rgba_to_minifb_buffer, qr_preview_square_side};
use crate::screen_bounds::{
    apply_anchored_preview_position, ensure_interactive_desktop, primary_work_area,
};

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

    let mut scaled_buffer = Vec::new();
    downscale_rgba_to_minifb_buffer(
        image.as_raw(),
        src_width,
        src_height,
        side,
        side,
        &mut scaled_buffer,
    );

    ensure_interactive_desktop();

    let mut window = Window::new(
        "VisioFlow QR",
        side as usize,
        side as usize,
        WindowOptions {
            topmost: true,
            resize: false,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| VisioFlowError::Encode(format!("failed to open QR preview window: {e}")))?;

    // Cap update rate at 60 FPS so the window loop doesn't pin 100% CPU on a core.
    window.set_target_fps(60);

    apply_anchored_preview_position(&mut window, &work_area, preview_position, side, side);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window
            .update_with_buffer(&scaled_buffer, side as usize, side as usize)
            .map_err(|e| VisioFlowError::Encode(format!("failed to update QR preview: {e}")))?;
    }

    Ok(())
}
