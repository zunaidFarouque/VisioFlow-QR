use image::{GrayImage, Luma};

/// Compute Otsu's optimal threshold for a grayscale image.
pub fn otsu_threshold(image: &GrayImage) -> u8 {
    let mut histogram = [0u32; 256];
    let total = (image.width() * image.height()) as f32;

    for pixel in image.pixels() {
        histogram[pixel[0] as usize] += 1;
    }

    let mut sum = 0f64;
    for (value, count) in histogram.iter().enumerate() {
        sum += value as f64 * *count as f64;
    }

    let mut sum_background = 0f64;
    let mut weight_background = 0u32;
    let mut max_variance = 0f64;
    let mut threshold = 0u8;

    for (t, &count) in histogram.iter().enumerate() {
        weight_background += count;
        if weight_background == 0 {
            continue;
        }

        let weight_foreground = image.width() * image.height() - weight_background;
        if weight_foreground == 0 {
            break;
        }

        sum_background += t as f64 * count as f64;
        let mean_background = sum_background / weight_background as f64;
        let mean_foreground = (sum - sum_background) / weight_foreground as f64;

        let weight_bg = weight_background as f64 / total as f64;
        let weight_fg = weight_foreground as f64 / total as f64;
        let variance = weight_bg * weight_fg * (mean_background - mean_foreground).powi(2);

        if variance > max_variance {
            max_variance = variance;
            threshold = t as u8;
        }
    }

    threshold
}

/// Binarize a grayscale image using Otsu's method.
pub fn binarize_otsu(image: &GrayImage) -> GrayImage {
    let threshold = otsu_threshold(image);
    let mut output = GrayImage::new(image.width(), image.height());

    for (x, y, pixel) in image.enumerate_pixels() {
        let value = if pixel[0] > threshold { 255 } else { 0 };
        output.put_pixel(x, y, Luma([value]));
    }

    output
}
