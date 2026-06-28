//! Manual exposure adjustment helpers.

/// Default arrow-key step size in EV stops.
pub const MANUAL_EV_STEP: f32 = 0.5;

/// Map a fractional EV offset to a clamped camera exposure integer.
#[must_use]
pub fn clamp_exposure_value_f32(
    baseline: i32,
    ev_offset: f32,
    min: i32,
    max: i32,
    step: i32,
) -> i32 {
    let step = step.max(1);
    let delta = (ev_offset * step as f32).round() as i32;
    let target = baseline.saturating_add(delta);
    target.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_ev_rounds_to_nearest_camera_step() {
        assert_eq!(clamp_exposure_value_f32(-6, 0.5, -12, 0, 1), -5);
        assert_eq!(clamp_exposure_value_f32(-6, 1.0, -12, 0, 1), -5);
        assert_eq!(clamp_exposure_value_f32(-6, -1.0, -12, 0, 1), -7);
    }
}
