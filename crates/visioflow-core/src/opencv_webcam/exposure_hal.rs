use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::error::Result;
use crate::opencv_webcam::exposure_table::{exposure_steps_for_backend, VideoBackend};
use crate::opencv_webcam::frame_stream::{cvprop, CaptureDriver, FrameStream};
use crate::traits::ExposureHal;

pub struct OpenCvExposureHal<D: CaptureDriver + 'static> {
    stream: Arc<FrameStream<D>>,
    steps: &'static [f64],
    current: AtomicUsize,
}

impl<D: CaptureDriver + 'static> OpenCvExposureHal<D> {
    #[must_use]
    pub fn new(stream: Arc<FrameStream<D>>, backend: VideoBackend) -> Self {
        Self {
            stream,
            steps: exposure_steps_for_backend(backend),
            current: AtomicUsize::new(0),
        }
    }

    #[must_use]
    pub fn stream(&self) -> Arc<FrameStream<D>> {
        Arc::clone(&self.stream)
    }

    pub fn enable_auto_exposure(&self) -> Result<()> {
        // OpenCV convention on Windows backends: 0.75 = auto, 0.25 = manual.
        let _ = self.stream.set_property(cvprop::CAP_PROP_AUTO_EXPOSURE, 0.75)?;
        Ok(())
    }
}

impl<D: CaptureDriver + 'static> ExposureHal for OpenCvExposureHal<D> {
    fn disable_auto_exposure(&self) -> Result<()> {
        let _ = self.stream.set_property(cvprop::CAP_PROP_AUTO_EXPOSURE, 0.25)?;
        Ok(())
    }

    fn set_step(&self, step_index: usize) -> Result<()> {
        let index = step_index.min(self.steps.len().saturating_sub(1));
        let value = self.steps[index];
        let _ = self.stream.set_property(cvprop::CAP_PROP_EXPOSURE, value)?;
        self.current.store(index, Ordering::Release);
        Ok(())
    }

    fn step_count(&self) -> usize {
        self.steps.len()
    }

    fn current_step(&self) -> usize {
        self.current.load(Ordering::Acquire)
    }
}
