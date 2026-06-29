#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoBackend {
    Dshow,
    Msmf,
    V4l2,
    Other,
}

const DSHOW_STEPS: &[f64] = &[-1.0, -3.0, -5.0, -7.0, -9.0, -11.0, -13.0];
const GENERIC_STEPS: &[f64] = &[-1.0, -4.0, -7.0, -10.0, -13.0];

#[must_use]
pub fn exposure_steps_for_backend(backend: VideoBackend) -> &'static [f64] {
    match backend {
        VideoBackend::Dshow => DSHOW_STEPS,
        VideoBackend::Msmf | VideoBackend::V4l2 | VideoBackend::Other => GENERIC_STEPS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dshow_table_matches_expected_values() {
        assert_eq!(
            exposure_steps_for_backend(VideoBackend::Dshow),
            &[-1.0, -3.0, -5.0, -7.0, -9.0, -11.0, -13.0]
        );
    }

    #[test]
    fn generic_table_matches_expected_values() {
        assert_eq!(
            exposure_steps_for_backend(VideoBackend::Msmf),
            &[-1.0, -4.0, -7.0, -10.0, -13.0]
        );
        assert_eq!(
            exposure_steps_for_backend(VideoBackend::V4l2),
            &[-1.0, -4.0, -7.0, -10.0, -13.0]
        );
    }

    #[test]
    fn all_tables_are_monotonic_brighter() {
        for backend in [VideoBackend::Dshow, VideoBackend::Msmf, VideoBackend::V4l2] {
            let steps = exposure_steps_for_backend(backend);
            for pair in steps.windows(2) {
                assert!(pair[0] > pair[1], "table must decrease toward darker values");
            }
        }
    }
}
