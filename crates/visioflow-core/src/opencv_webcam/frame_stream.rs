use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::error::{Result, VisioFlowError};
use crate::traits::{BgrFrame, LiveFrameSource};

pub trait CaptureDriver: Send {
    fn grab(&mut self) -> Result<bool>;
    fn retrieve_bgr(&mut self) -> Result<BgrFrame>;
    fn set_property(&mut self, property: i32, value: f64) -> Result<bool>;
}

pub mod cvprop {
    #[cfg(feature = "opencv-webcam")]
    pub const CAP_PROP_AUTO_EXPOSURE: i32 = opencv::videoio::CAP_PROP_AUTO_EXPOSURE;
    #[cfg(not(feature = "opencv-webcam"))]
    pub const CAP_PROP_AUTO_EXPOSURE: i32 = 21;

    #[cfg(feature = "opencv-webcam")]
    pub const CAP_PROP_EXPOSURE: i32 = opencv::videoio::CAP_PROP_EXPOSURE;
    #[cfg(not(feature = "opencv-webcam"))]
    pub const CAP_PROP_EXPOSURE: i32 = 15;
}

struct Inner<D: CaptureDriver + 'static> {
    driver: Mutex<D>,
    grab_count: AtomicU64,
    grab_pair: (Mutex<()>, Condvar),
    stop: AtomicBool,
}

pub struct FrameStream<D: CaptureDriver + 'static> {
    inner: Arc<Inner<D>>,
    spin_thread: Mutex<Option<JoinHandle<()>>>,
}

impl<D: CaptureDriver + 'static> FrameStream<D> {
    pub fn start(driver: D) -> Self {
        let inner = Arc::new(Inner {
            driver: Mutex::new(driver),
            grab_count: AtomicU64::new(0),
            grab_pair: (Mutex::new(()), Condvar::new()),
            stop: AtomicBool::new(false),
        });
        let thread_inner = Arc::clone(&inner);
        let handle = thread::spawn(move || {
            while !thread_inner.stop.load(Ordering::Relaxed) {
                let grabbed = {
                    let mut driver = thread_inner.driver.lock().expect("lock capture driver");
                    driver.grab().unwrap_or(false)
                };
                if grabbed {
                    thread_inner.grab_count.fetch_add(1, Ordering::Release);
                    thread_inner.grab_pair.1.notify_all();
                } else {
                    thread::sleep(Duration::from_millis(2));
                }
            }
        });
        Self {
            inner,
            spin_thread: Mutex::new(Some(handle)),
        }
    }

    pub fn set_property(&self, property: i32, value: f64) -> Result<bool> {
        let mut driver = self.inner.driver.lock().map_err(|_| {
            VisioFlowError::Capture("failed to lock capture driver for set_property".into())
        })?;
        driver.set_property(property, value)
    }
}

impl<D: CaptureDriver + 'static> LiveFrameSource for FrameStream<D> {
    fn latest_frame(&self) -> Result<BgrFrame> {
        let mut driver = self.inner.driver.lock().map_err(|_| {
            VisioFlowError::Capture("failed to lock capture driver for retrieve".into())
        })?;
        driver.retrieve_bgr()
    }

    fn flush_after_exposure_change(&self, grabs: u32) -> Result<()> {
        let start = self.inner.grab_count.load(Ordering::Acquire);
        let target = start.saturating_add(u64::from(grabs));
        let mut guard = self.inner.grab_pair.0.lock().map_err(|_| {
            VisioFlowError::Capture("failed to lock frame flush condition".into())
        })?;
        while self.inner.grab_count.load(Ordering::Acquire) < target {
            guard = self
                .inner
                .grab_pair
                .1
                .wait_timeout(guard, Duration::from_millis(20))
                .map_err(|_| VisioFlowError::Capture("frame flush wait failed".into()))?
                .0;
        }
        Ok(())
    }
}

impl<D: CaptureDriver + 'static> LiveFrameSource for Arc<FrameStream<D>> {
    fn latest_frame(&self) -> Result<BgrFrame> {
        self.as_ref().latest_frame()
    }

    fn flush_after_exposure_change(&self, grabs: u32) -> Result<()> {
        self.as_ref().flush_after_exposure_change(grabs)
    }
}

impl<D: CaptureDriver + 'static> Drop for FrameStream<D> {
    fn drop(&mut self) {
        self.inner.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.spin_thread.lock().ok().and_then(|mut h| h.take()) {
            let _ = handle.join();
        }
    }
}

#[cfg(feature = "opencv-webcam")]
pub struct OpenCvCaptureDriver {
    capture: opencv::videoio::VideoCapture,
}

#[cfg(feature = "opencv-webcam")]
impl OpenCvCaptureDriver {
    pub fn open_default() -> Result<Self> {
        use opencv::prelude::VideoCaptureTraitConst;
        use opencv::videoio::{VideoCapture, CAP_ANY, CAP_DSHOW, CAP_MSMF, CAP_V4L2};

        let backends: &[i32] = if cfg!(target_os = "windows") {
            &[CAP_DSHOW, CAP_MSMF, CAP_ANY]
        } else if cfg!(target_os = "linux") {
            &[CAP_V4L2, CAP_ANY]
        } else {
            &[CAP_ANY]
        };

        for backend in backends {
            let capture = VideoCapture::new(0, *backend)
                .map_err(|error| VisioFlowError::Capture(format!("failed to open camera: {error}")))?;
            if capture
                .is_opened()
                .map_err(|error| VisioFlowError::Capture(format!("opencv is_opened failed: {error}")))?
            {
                return Ok(Self { capture });
            }
        }
        Err(VisioFlowError::Capture(
            "failed to open OpenCV webcam on any backend".into(),
        ))
    }
}

#[cfg(feature = "opencv-webcam")]
impl CaptureDriver for OpenCvCaptureDriver {
    fn grab(&mut self) -> Result<bool> {
        use opencv::prelude::VideoCaptureTrait;
        self.capture
            .grab()
            .map_err(|error| VisioFlowError::Capture(format!("opencv grab failed: {error}")))
    }

    fn retrieve_bgr(&mut self) -> Result<BgrFrame> {
        use opencv::core::Mat;
        use opencv::prelude::{MatTraitConst, MatTraitConstManual, VideoCaptureTrait};

        let mut mat = Mat::default();
        let ok = self
            .capture
            .retrieve(&mut mat, 0)
            .map_err(|error| VisioFlowError::Capture(format!("opencv retrieve failed: {error}")))?;
        if !ok || mat.empty() {
            return Err(VisioFlowError::Capture(
                "opencv retrieve produced empty frame".into(),
            ));
        }
        let width = mat.cols() as u32;
        let height = mat.rows() as u32;
        let data = mat
            .data_bytes()
            .map_err(|error| VisioFlowError::Capture(format!("opencv data_bytes failed: {error}")))?
            .to_vec();
        Ok(BgrFrame::new(width, height, data))
    }

    fn set_property(&mut self, property: i32, value: f64) -> Result<bool> {
        use opencv::prelude::VideoCaptureTrait;
        self.capture
            .set(property, value)
            .map_err(|error| VisioFlowError::Capture(format!("opencv set property failed: {error}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct FakeCaptureDriver {
        count: Arc<AtomicUsize>,
    }

    impl FakeCaptureDriver {
        fn new() -> Self {
            Self {
                count: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl CaptureDriver for FakeCaptureDriver {
        fn grab(&mut self) -> Result<bool> {
            self.count.fetch_add(1, Ordering::Relaxed);
            Ok(true)
        }

        fn retrieve_bgr(&mut self) -> Result<BgrFrame> {
            Ok(BgrFrame::new(1, 1, vec![0, 0, 0]))
        }

        fn set_property(&mut self, _property: i32, _value: f64) -> Result<bool> {
            Ok(true)
        }
    }

    #[test]
    fn flush_waits_for_requested_grabs() {
        let stream = FrameStream::start(FakeCaptureDriver::new());
        stream
            .flush_after_exposure_change(3)
            .expect("flush should complete");
    }

    #[test]
    fn latest_frame_retrieves_data() {
        let stream = FrameStream::start(FakeCaptureDriver::new());
        let frame = stream.latest_frame().expect("frame");
        assert_eq!(frame.width, 1);
        assert_eq!(frame.height, 1);
        assert_eq!(frame.data, vec![0, 0, 0]);
    }
}
