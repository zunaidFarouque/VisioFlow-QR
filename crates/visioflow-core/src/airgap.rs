use crate::error::{Result, VisioFlowError};

/// Returns true when air-gap mode is requested via CLI or environment.
pub fn airgap_active(disable_telemetry: bool) -> bool {
    disable_telemetry || airgap_env_active()
}

fn airgap_env_active() -> bool {
    std::env::var("VISIOFLOW_AIRGAP")
        .map(|value| value == "1")
        .unwrap_or(false)
}

/// Refuses startup when air-gap mode is active (blocks future OTLP initialization).
pub fn enforce_airgap_policy(disable_telemetry: bool) -> Result<()> {
    if airgap_active(disable_telemetry) {
        Err(VisioFlowError::AirGap)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn airgap_active_when_disable_telemetry_flag_set() {
        assert!(airgap_active(true));
    }

    #[test]
    fn airgap_inactive_without_flag_or_env() {
        let _guard = EnvGuard::unset("VISIOFLOW_AIRGAP");
        assert!(!airgap_active(false));
    }

    #[test]
    fn airgap_active_when_env_var_is_one() {
        let _guard = EnvGuard::set("VISIOFLOW_AIRGAP", "1");
        assert!(airgap_active(false));
    }

    #[test]
    fn enforce_airgap_policy_errors_in_airgap_mode() {
        let err = enforce_airgap_policy(true).expect_err("expected air-gap error");
        assert!(matches!(err, VisioFlowError::AirGap));
    }

    #[test]
    fn enforce_airgap_policy_ok_when_not_airgapped() {
        let _guard = EnvGuard::unset("VISIOFLOW_AIRGAP");
        enforce_airgap_policy(false).expect("should allow startup");
    }

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            // SAFETY: test-only; single-threaded cargo test workers per crate.
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }

        fn unset(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            // SAFETY: test-only.
            unsafe { std::env::remove_var(key) };
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }
}
