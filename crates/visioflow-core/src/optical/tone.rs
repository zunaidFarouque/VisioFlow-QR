//! Software exposure / tone adjustment for manual EV and decode.

use image::{DynamicImage, GrayImage, Luma, Rgb, Rgba};

/// Apply an EV stop adjustment to image luminance (multiply by 2^ev, clamp to 0..255).
#[must_use]
pub fn apply_ev_adjustment_f32(image: &DynamicImage, ev_stop: f32) -> DynamicImage {
    if ev_stop.abs() < f32::EPSILON {
        return image.clone();
    }

    let scale = 2.0_f32.powf(ev_stop);
    match image {
        DynamicImage::ImageLuma8(gray) => DynamicImage::ImageLuma8(adjust_gray(gray, scale)),
        DynamicImage::ImageLumaA8(gray_a) => {
            let mut out = gray_a.clone();
            for pixel in out.pixels_mut() {
                pixel.0[0] = scale_luma(pixel.0[0], scale);
            }
            DynamicImage::ImageLumaA8(out)
        }
        DynamicImage::ImageRgb8(rgb) => DynamicImage::ImageRgb8(adjust_rgb(rgb, scale)),
        DynamicImage::ImageRgba8(rgba) => DynamicImage::ImageRgba8(adjust_rgba(rgba, scale)),
        other => {
            let rgb = other.to_rgb8();
            DynamicImage::ImageRgb8(adjust_rgb(&rgb, scale))
        }
    }
}

/// Integer EV helper (whole stops).
#[must_use]
pub fn apply_ev_adjustment(image: &DynamicImage, ev_stop: i8) -> DynamicImage {
    apply_ev_adjustment_f32(image, f32::from(ev_stop))
}

fn adjust_gray(gray: &GrayImage, scale: f32) -> GrayImage {
    GrayImage::from_fn(gray.width(), gray.height(), |x, y| {
        Luma([scale_luma(gray.get_pixel(x, y).0[0], scale)])
    })
}

fn adjust_rgb(rgb: &image::RgbImage, scale: f32) -> image::RgbImage {
    image::RgbImage::from_fn(rgb.width(), rgb.height(), |x, y| {
        let p = rgb.get_pixel(x, y);
        Rgb([
            scale_luma(p.0[0], scale),
            scale_luma(p.0[1], scale),
            scale_luma(p.0[2], scale),
        ])
    })
}

fn adjust_rgba(rgba: &image::RgbaImage, scale: f32) -> image::RgbaImage {
    image::RgbaImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let p = rgba.get_pixel(x, y);
        Rgba([
            scale_luma(p.0[0], scale),
            scale_luma(p.0[1], scale),
            scale_luma(p.0[2], scale),
            p.0[3],
        ])
    })
}

fn scale_luma(value: u8, scale: f32) -> u8 {
    (f32::from(value) * scale).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma};

    #[test]
    fn ev_zero_is_identity() {
        let gray = GrayImage::from_pixel(2, 2, Luma([100]));
        let image = DynamicImage::ImageLuma8(gray.clone());
        let adjusted = apply_ev_adjustment_f32(&image, 0.0);
        assert_eq!(adjusted.as_bytes(), gray.as_raw());
    }

    #[test]
    fn ev_plus_one_doubles_dark_pixels() {
        let gray = GrayImage::from_pixel(1, 1, Luma([50]));
        let image = DynamicImage::ImageLuma8(gray);
        let adjusted = apply_ev_adjustment_f32(&image, 1.0);
        assert_eq!(adjusted.as_bytes(), &[100]);
    }

    #[test]
    fn ev_half_stop_scales_by_sqrt_two() {
        let gray = GrayImage::from_pixel(1, 1, Luma([100]));
        let image = DynamicImage::ImageLuma8(gray);
        let adjusted = apply_ev_adjustment_f32(&image, 0.5);
        assert_eq!(adjusted.as_bytes(), &[141]);
    }

    #[test]
    fn ev_minus_one_halves_bright_pixels() {
        let gray = GrayImage::from_pixel(1, 1, Luma([200]));
        let image = DynamicImage::ImageLuma8(gray);
        let adjusted = apply_ev_adjustment_f32(&image, -1.0);
        assert_eq!(adjusted.as_bytes(), &[100]);
    }

    #[test]
    fn ev_clamps_to_byte_range() {
        let gray = GrayImage::from_pixel(1, 1, Luma([200]));
        let image = DynamicImage::ImageLuma8(gray);
        let adjusted = apply_ev_adjustment_f32(&image, 2.0);
        assert_eq!(adjusted.as_bytes(), &[255]);
    }
}
