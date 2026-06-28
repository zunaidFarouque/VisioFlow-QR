use image::DynamicImage;

/// Maximum frame width (~300 DPI equivalent for typical screen captures).
pub const MAX_FRAME_WIDTH: u32 = 1200;

/// Downsample frames wider than `max_width`, preserving aspect ratio.
pub fn downsample(image: &DynamicImage, max_width: u32) -> DynamicImage {
    if image.width() <= max_width {
        return image.clone();
    }

    let scale = max_width as f32 / image.width() as f32;
    let new_height = (image.height() as f32 * scale).round() as u32;
    image.resize_exact(max_width, new_height, image::imageops::FilterType::Triangle)
}
