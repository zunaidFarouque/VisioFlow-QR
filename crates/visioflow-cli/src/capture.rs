use image::DynamicImage;
use visioflow_core::error::{Result, VisioFlowError};
use visioflow_core::traits::FrameSource;

/// Captures the primary monitor screen as a frame.
pub struct SnipFrameSource;

impl FrameSource for SnipFrameSource {
    fn capture_frame(&self) -> Result<DynamicImage> {
        let monitors = xcap::Monitor::all()
            .map_err(|e| VisioFlowError::Capture(format!("failed to enumerate monitors: {e}")))?;

        let monitor = monitors.into_iter().next().ok_or_else(|| {
            VisioFlowError::Capture("no monitors available for screen capture".into())
        })?;

        let rgba = monitor
            .capture_image()
            .map_err(|e| VisioFlowError::Capture(format!("screen capture failed: {e}")))?;

        Ok(DynamicImage::ImageRgba8(rgba))
    }
}

/// Loads a frame from disk (used for integration tests and debugging).
pub struct FileFrameSource {
    path: std::path::PathBuf,
}

impl FileFrameSource {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl FrameSource for FileFrameSource {
    fn capture_frame(&self) -> Result<DynamicImage> {
        image::open(&self.path).map_err(VisioFlowError::from)
    }
}
