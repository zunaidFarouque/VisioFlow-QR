use image::{DynamicImage, GrayImage};
use std::time::Instant;

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

/// Raw BGR frame used by OpenCV webcam scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgrFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl BgrFrame {
    #[must_use]
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
        }
    }
}

/// Live frame source that returns the freshest webcam frame.
#[cfg_attr(test, mockall::automock)]
pub trait LiveFrameSource: Send + Sync {
    fn latest_frame(&self) -> Result<BgrFrame>;
    fn flush_after_exposure_change(&self, grabs: u32) -> Result<()>;
}

/// CNN-based QR decoder for live BGR frames.
#[cfg_attr(test, mockall::automock)]
pub trait CnnQrDecoder: Send + Sync {
    fn decode_bgr(&self, frame: &BgrFrame) -> Result<Vec<String>>;
}

/// Hardware abstraction for webcam exposure bracketing.
#[cfg_attr(test, mockall::automock)]
pub trait ExposureHal: Send + Sync {
    fn disable_auto_exposure(&self) -> Result<()>;
    fn set_step(&self, step_index: usize) -> Result<()>;
    fn step_count(&self) -> usize;
    fn current_step(&self) -> usize;
}

/// End-to-end live optical scanner API.
#[cfg_attr(test, mockall::automock)]
pub trait OpticalScanner: Send + Sync {
    fn scan_until(&self, deadline: Instant) -> Result<Vec<String>>;
}
