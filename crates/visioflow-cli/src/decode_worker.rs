//! Background thread that runs CNN QR decode off the preview loop.

use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use visioflow_core::error::VisioFlowError;
use visioflow_core::traits::{BgrFrame, CnnQrDecoder};

#[derive(Debug)]
pub enum DecodeOutcome {
    Success(Vec<String>),
    NoPayloads,
    Failed(VisioFlowError),
}

enum WorkerCommand {
    Decode(BgrFrame),
    Shutdown,
}

pub struct AsyncDecodeWorker {
    request_tx: SyncSender<WorkerCommand>,
    result_rx: Receiver<DecodeOutcome>,
    thread: Option<JoinHandle<()>>,
}

impl AsyncDecodeWorker {
    pub fn spawn<D: CnnQrDecoder + 'static>(decoder: Arc<D>) -> Self {
        let (request_tx, request_rx) = mpsc::sync_channel(1);
        let (result_tx, result_rx) = mpsc::channel();

        let thread = thread::spawn(move || {
            while let Ok(command) = request_rx.recv() {
                match command {
                    WorkerCommand::Shutdown => break,
                    WorkerCommand::Decode(frame) => {
                        let outcome = match decoder.decode_bgr(&frame) {
                            Ok(payloads) if !payloads.is_empty() => DecodeOutcome::Success(payloads),
                            Ok(_) | Err(VisioFlowError::NoPayloads) => DecodeOutcome::NoPayloads,
                            Err(error) => DecodeOutcome::Failed(error),
                        };
                        let _ = result_tx.send(outcome);
                    }
                }
            }
        });

        Self {
            request_tx,
            result_rx,
            thread: Some(thread),
        }
    }

    /// Submit a frame for decode. Returns false when the worker is still busy.
    pub fn try_submit(&self, frame: BgrFrame) -> bool {
        match self.request_tx.try_send(WorkerCommand::Decode(frame)) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => false,
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    pub fn try_recv(&self) -> Option<DecodeOutcome> {
        match self.result_rx.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }
}

impl Drop for AsyncDecodeWorker {
    fn drop(&mut self) {
        let _ = self.request_tx.try_send(WorkerCommand::Shutdown);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use visioflow_core::error::Result;

    struct MockDecoder {
        remaining_failures: Mutex<usize>,
        decode_calls: AtomicUsize,
    }

    impl MockDecoder {
        fn fail_then_succeed(failures: usize) -> Arc<Self> {
            Arc::new(Self {
                remaining_failures: Mutex::new(failures),
                decode_calls: AtomicUsize::new(0),
            })
        }
    }

    impl CnnQrDecoder for MockDecoder {
        fn decode_bgr(&self, _frame: &BgrFrame) -> Result<Vec<String>> {
            self.decode_calls.fetch_add(1, Ordering::SeqCst);
            let mut left = self.remaining_failures.lock().expect("lock failures");
            if *left == 0 {
                return Ok(vec!["payload".to_string()]);
            }
            *left -= 1;
            Err(VisioFlowError::NoPayloads)
        }
    }

    struct SlowDecoder;

    impl CnnQrDecoder for SlowDecoder {
        fn decode_bgr(&self, _frame: &BgrFrame) -> Result<Vec<String>> {
            thread::sleep(Duration::from_millis(200));
            Err(VisioFlowError::NoPayloads)
        }
    }

    fn frame() -> BgrFrame {
        BgrFrame::new(1, 1, vec![0, 0, 0])
    }

    fn wait_for_outcome(worker: &AsyncDecodeWorker) -> DecodeOutcome {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Some(outcome) = worker.try_recv() {
                return outcome;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("timed out waiting for decode outcome");
    }

    #[test]
    fn worker_returns_success_from_background_thread() {
        let worker = AsyncDecodeWorker::spawn(MockDecoder::fail_then_succeed(0));
        assert!(worker.try_submit(frame()));
        match wait_for_outcome(&worker) {
            DecodeOutcome::Success(payloads) => assert_eq!(payloads, vec!["payload".to_string()]),
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test]
    fn worker_returns_no_payloads() {
        let worker = AsyncDecodeWorker::spawn(MockDecoder::fail_then_succeed(1));
        assert!(worker.try_submit(frame()));
        assert!(matches!(
            wait_for_outcome(&worker),
            DecodeOutcome::NoPayloads
        ));
    }

    #[test]
    fn try_submit_returns_false_when_worker_busy() {
        let worker = AsyncDecodeWorker::spawn(Arc::new(SlowDecoder));
        assert!(worker.try_submit(frame()));
        assert!(!worker.try_submit(frame()));
        let _ = wait_for_outcome(&worker);
    }
}
