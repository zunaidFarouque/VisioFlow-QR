use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};

use crate::optical::downsample;

const MAX_FRAME_WIDTH: u32 = 1200;

#[test]
fn downsample_reduces_1920x1080_to_target_bounds() {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_fn(1920, 1080, |_, _| {
        Rgba([0, 0, 0, 255])
    }));

    let result = downsample(&image, MAX_FRAME_WIDTH);
    assert!(result.width() <= MAX_FRAME_WIDTH);
    assert!(result.height() < 1080);
    assert_eq!(
        result.width(),
        MAX_FRAME_WIDTH,
        "width should be capped at max frame width"
    );
}

#[test]
fn downsample_leaves_small_images_unchanged() {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_fn(640, 480, |_, _| {
        Rgba([255, 255, 255, 255])
    }));

    let result = downsample(&image, MAX_FRAME_WIDTH);
    assert_eq!(result.dimensions(), (640, 480));
}
