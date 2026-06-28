use image::GrayImage;

use crate::optical::{binarize_otsu, otsu_threshold};

#[test]
fn otsu_threshold_on_bimodal_8x8_image() {
    // Left half dark (30), right half bright (220).
    let pixels: Vec<u8> = (0..64)
        .map(|i| {
            if i % 8 < 4 {
                30
            } else {
                220
            }
        })
        .collect();
    let image = GrayImage::from_raw(8, 8, pixels).expect("valid 8x8 image");

    let threshold = otsu_threshold(&image);
    assert!(
        threshold == 30 || (100..=150).contains(&threshold),
        "expected threshold at class boundary, got {threshold}"
    );
}

#[test]
fn otsu_binarize_produces_expected_binary_pattern() {
    let pixels: Vec<u8> = (0..64)
        .map(|i| {
            if i % 8 < 4 {
                30
            } else {
                220
            }
        })
        .collect();
    let image = GrayImage::from_raw(8, 8, pixels).expect("valid 8x8 image");

    let binary = binarize_otsu(&image);
    for (idx, pixel) in binary.pixels().enumerate() {
        let col = idx % 8;
        let expected = if col < 4 { 0 } else { 255 };
        assert_eq!(pixel[0], expected, "pixel {idx} should be {expected}");
    }
}

#[test]
fn otsu_on_uniform_image_returns_valid_threshold() {
    let pixels = vec![128u8; 64];
    let image = GrayImage::from_raw(8, 8, pixels).expect("valid 8x8 image");
    let threshold = otsu_threshold(&image);
    let binary = binarize_otsu(&image);

    let _ = threshold;
    let unique: std::collections::HashSet<u8> = binary.pixels().map(|p| p[0]).collect();
    assert_eq!(unique.len(), 1, "uniform input should produce uniform binary output");
}
