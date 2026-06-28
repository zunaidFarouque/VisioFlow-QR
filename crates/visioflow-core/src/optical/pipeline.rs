use image::GrayImage;
use imageproc::filter::median_filter;

use crate::traits::OpticalFilterKind;

use super::downsample::downsample;
use super::otsu::binarize_otsu;

/// Run the optical preprocessing pipeline: optional median blur, then Otsu binarization.
pub fn run_optical_pipeline(image: &GrayImage, filter: OpticalFilterKind) -> GrayImage {
    let filtered = preprocess_grayscale(image, filter);
    binarize_otsu(&filtered)
}

/// Downsample and optionally median-filter without binarizing (decoder fallback path).
pub fn preprocess_grayscale(image: &GrayImage, filter: OpticalFilterKind) -> GrayImage {
    match filter {
        OpticalFilterKind::Otsu => image.clone(),
        OpticalFilterKind::Median => median_filter(image, 1, 1),
    }
}

/// Convert a dynamic image to grayscale, downsample, and run the optical pipeline.
pub fn preprocess_frame(
    image: &image::DynamicImage,
    max_width: u32,
    filter: OpticalFilterKind,
) -> GrayImage {
    let downsampled = downsample(image, max_width);
    let gray = downsampled.to_luma8();
    run_optical_pipeline(&gray, filter)
}

/// Downsample to grayscale with optional median filtering (no Otsu).
pub fn preprocess_frame_grayscale(
    image: &image::DynamicImage,
    max_width: u32,
    filter: OpticalFilterKind,
) -> GrayImage {
    let downsampled = downsample(image, max_width);
    let gray = downsampled.to_luma8();
    preprocess_grayscale(&gray, filter)
}
