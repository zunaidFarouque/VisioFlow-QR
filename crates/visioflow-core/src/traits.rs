use image::{DynamicImage, GrayImage};

use crate::error::Result;

/// Optical preprocessing filter applied before binarization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpticalFilterKind {
    #[default]
    Otsu,
    Median,
}

/// Captures a frame from a physical or screen source.
#[cfg_attr(test, mockall::automock)]
pub trait FrameSource: Send + Sync {
    fn capture_frame(&self) -> Result<DynamicImage>;
}

/// Decodes visual payloads from a preprocessed grayscale image.
#[cfg_attr(test, mockall::automock)]
pub trait PayloadDecoder: Send + Sync {
    fn decode(&self, image: &GrayImage) -> Result<Vec<String>>;
}
