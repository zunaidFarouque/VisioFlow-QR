use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct BracketConfig {
    pub primary_timeout: Duration,
    pub flush_grabs: u32,
}

impl Default for BracketConfig {
    fn default() -> Self {
        Self {
            primary_timeout: Duration::from_millis(500),
            flush_grabs: 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BracketAction {
    KeepPrimary,
    AdvanceExposureStep { step_index: usize, flush_grabs: u32 },
    Exhausted,
}

#[derive(Debug, Clone)]
pub struct BracketState {
    config: BracketConfig,
    started_at: Instant,
    current_step: usize,
    max_steps: usize,
}

impl BracketState {
    #[must_use]
    pub fn new(config: BracketConfig, started_at: Instant, max_steps: usize) -> Self {
        Self {
            config,
            started_at,
            current_step: 0,
            max_steps,
        }
    }

    #[must_use]
    pub fn current_step(&self) -> usize {
        self.current_step
    }

    pub fn on_primary_decode_failure(&mut self, now: Instant) -> BracketAction {
        if now.duration_since(self.started_at) < self.config.primary_timeout {
            return BracketAction::KeepPrimary;
        }
        if self.current_step >= self.max_steps {
            return BracketAction::Exhausted;
        }
        let action = BracketAction::AdvanceExposureStep {
            step_index: self.current_step,
            flush_grabs: self.config.flush_grabs,
        };
        self.current_step += 1;
        self.started_at = now;
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_timeout_waits_before_stepping() {
        let started = Instant::now();
        let mut state = BracketState::new(
            BracketConfig {
                primary_timeout: Duration::from_millis(500),
                flush_grabs: 5,
            },
            started,
            3,
        );
        let action = state.on_primary_decode_failure(started + Duration::from_millis(499));
        assert_eq!(action, BracketAction::KeepPrimary);
    }

    #[test]
    fn timeout_advances_step_with_flush() {
        let started = Instant::now();
        let mut state = BracketState::new(
            BracketConfig {
                primary_timeout: Duration::from_millis(500),
                flush_grabs: 5,
            },
            started,
            3,
        );
        let action = state.on_primary_decode_failure(started + Duration::from_millis(500));
        assert_eq!(
            action,
            BracketAction::AdvanceExposureStep {
                step_index: 0,
                flush_grabs: 5
            }
        );
        assert_eq!(state.current_step(), 1);
    }

    #[test]
    fn exhausted_after_all_steps_used() {
        let started = Instant::now();
        let mut state = BracketState::new(BracketConfig::default(), started, 1);
        let first = state.on_primary_decode_failure(started + Duration::from_millis(501));
        assert!(matches!(
            first,
            BracketAction::AdvanceExposureStep { step_index: 0, .. }
        ));

        let second = state.on_primary_decode_failure(
            started + Duration::from_millis(501) + Duration::from_millis(501),
        );
        assert_eq!(second, BracketAction::Exhausted);
    }
}
