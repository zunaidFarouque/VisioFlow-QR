use std::time::Instant;

use crate::error::{Result, VisioFlowError};
use crate::opencv_webcam::bracket::{BracketAction, BracketConfig, BracketState};
use crate::traits::{CnnQrDecoder, ExposureHal, LiveFrameSource, OpticalScanner};

pub struct TemporalBracketScanner<F, D, E> {
    frame_source: F,
    decoder: D,
    exposure_hal: E,
    config: BracketConfig,
}

impl<F, D, E> TemporalBracketScanner<F, D, E> {
    #[must_use]
    pub fn new(frame_source: F, decoder: D, exposure_hal: E, config: BracketConfig) -> Self {
        Self {
            frame_source,
            decoder,
            exposure_hal,
            config,
        }
    }
}

impl<F, D, E> OpticalScanner for TemporalBracketScanner<F, D, E>
where
    F: LiveFrameSource,
    D: CnnQrDecoder,
    E: ExposureHal,
{
    fn scan_until(&self, deadline: Instant) -> Result<Vec<String>> {
        self.exposure_hal.disable_auto_exposure()?;
        let mut bracket =
            BracketState::new(self.config, Instant::now(), self.exposure_hal.step_count());

        while Instant::now() < deadline {
            let frame = self.frame_source.latest_frame()?;
            match self.decoder.decode_bgr(&frame) {
                Ok(payloads) if !payloads.is_empty() => return Ok(payloads),
                Ok(_) | Err(VisioFlowError::NoPayloads) => {
                    match bracket.on_primary_decode_failure(Instant::now()) {
                        BracketAction::KeepPrimary => {}
                        BracketAction::AdvanceExposureStep {
                            step_index,
                            flush_grabs,
                        } => {
                            self.exposure_hal.set_step(step_index)?;
                            self.frame_source.flush_after_exposure_change(flush_grabs)?;
                        }
                        BracketAction::Exhausted => break,
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Err(VisioFlowError::NoPayloads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use crate::traits::BgrFrame;

    struct MockLiveFrameSource {
        frames: Arc<Mutex<Vec<BgrFrame>>>,
        flush_calls: Arc<Mutex<Vec<u32>>>,
    }

    impl MockLiveFrameSource {
        fn new(frames: Vec<BgrFrame>) -> Self {
            Self {
                frames: Arc::new(Mutex::new(frames)),
                flush_calls: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl LiveFrameSource for MockLiveFrameSource {
        fn latest_frame(&self) -> Result<BgrFrame> {
            self.frames
                .lock()
                .expect("lock frames")
                .pop()
                .ok_or(VisioFlowError::NoPayloads)
        }

        fn flush_after_exposure_change(&self, grabs: u32) -> Result<()> {
            self.flush_calls.lock().expect("lock flush").push(grabs);
            Ok(())
        }
    }

    struct MockCnnQrDecoder {
        fail_count: Arc<Mutex<usize>>,
    }

    impl MockCnnQrDecoder {
        fn new(fail_count: usize) -> Self {
            Self {
                fail_count: Arc::new(Mutex::new(fail_count)),
            }
        }
    }

    impl CnnQrDecoder for MockCnnQrDecoder {
        fn decode_bgr(&self, _frame: &BgrFrame) -> Result<Vec<String>> {
            let mut left = self.fail_count.lock().expect("lock fail_count");
            if *left == 0 {
                return Ok(vec!["ok".to_string()]);
            }
            *left -= 1;
            Err(VisioFlowError::NoPayloads)
        }
    }

    struct MockExposureHal {
        steps: usize,
        set_calls: Arc<Mutex<Vec<usize>>>,
    }

    impl MockExposureHal {
        fn new(steps: usize) -> Self {
            Self {
                steps,
                set_calls: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl ExposureHal for MockExposureHal {
        fn disable_auto_exposure(&self) -> Result<()> {
            Ok(())
        }

        fn set_step(&self, step_index: usize) -> Result<()> {
            self.set_calls
                .lock()
                .expect("lock set_calls")
                .push(step_index);
            Ok(())
        }

        fn step_count(&self) -> usize {
            self.steps
        }

        fn current_step(&self) -> usize {
            *self
                .set_calls
                .lock()
                .expect("lock set calls")
                .last()
                .unwrap_or(&0)
        }
    }

    fn frame() -> BgrFrame {
        BgrFrame::new(1, 1, vec![0, 0, 0])
    }

    #[test]
    fn scan_returns_when_decoder_succeeds() {
        let source = MockLiveFrameSource::new(vec![frame(), frame(), frame(), frame()]);
        let decoder = MockCnnQrDecoder::new(1);
        let exposure = MockExposureHal::new(4);
        let scanner = TemporalBracketScanner::new(
            source,
            decoder,
            exposure,
            BracketConfig {
                primary_timeout: Duration::from_millis(1),
                flush_grabs: 5,
            },
        );

        let result = scanner.scan_until(Instant::now() + Duration::from_millis(20));
        assert_eq!(result.expect("payload"), vec!["ok".to_string()]);
    }

    #[test]
    fn scan_brackets_and_times_out() {
        let source = MockLiveFrameSource::new(vec![frame(); 20]);
        let decoder = MockCnnQrDecoder::new(usize::MAX / 2);
        let exposure = MockExposureHal::new(2);
        let set_calls = Arc::clone(&exposure.set_calls);
        let flush_calls = Arc::clone(&source.flush_calls);
        let scanner = TemporalBracketScanner::new(
            source,
            decoder,
            exposure,
            BracketConfig {
                primary_timeout: Duration::from_millis(0),
                flush_grabs: 3,
            },
        );

        let result = scanner.scan_until(Instant::now() + Duration::from_millis(20));
        assert!(matches!(result, Err(VisioFlowError::NoPayloads)));
        assert_eq!(*set_calls.lock().expect("set calls"), vec![0, 1]);
        assert_eq!(*flush_calls.lock().expect("flush calls"), vec![3, 3]);
    }
}
