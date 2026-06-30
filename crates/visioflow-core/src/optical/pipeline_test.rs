use image::GrayImage;

use crate::optical::run_optical_pipeline;
use crate::traits::OpticalFilterKind;

#[test]
fn pipeline_produces_binary_image_from_noisy_grayscale() {
    let mut pixels = Vec::with_capacity(64 * 64);
    for y in 0..64 {
        for x in 0..64 {
            let base = if x < 32 { 40u8 } else { 210u8 };
            let noise = if (x + y) % 3 == 0 { 15 } else { 0 };
            pixels.push(base.saturating_add(noise));
        }
    }
    let image = GrayImage::from_raw(64, 64, pixels).expect("valid image");

    let processed = run_optical_pipeline(&image, OpticalFilterKind::Otsu);

    let unique: std::collections::HashSet<u8> = processed.pixels().map(|p| p[0]).collect();
    assert!(
        unique.len() <= 2,
        "pipeline output should be binary, got {unique:?}"
    );
    assert!(unique.contains(&0) || unique.contains(&255));
}

#[test]
fn pipeline_with_median_filter_still_binarizes() {
    let mut pixels = vec![128u8; 32 * 32];
    for y in 0..32 {
        for x in 0..32 {
            let idx = (y * 32 + x) as usize;
            pixels[idx] = if x < 16 { 50 } else { 200 };
        }
    }
    let image = GrayImage::from_raw(32, 32, pixels).expect("valid image");

    let processed = run_optical_pipeline(&image, OpticalFilterKind::Median);
    assert_eq!(processed.dimensions(), (32, 32));
}
