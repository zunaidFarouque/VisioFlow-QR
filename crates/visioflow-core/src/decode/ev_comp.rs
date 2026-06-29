//! EV compensation step math (platform-agnostic).

/// Windows `KSCAMERA_EXTENDEDPROP_EVCOMP_SIXTHSTEP` (1/6 EV per step unit).
pub const EV_COMP_FLAG_SIXTH_STEP: u64 = 1;
/// Windows `KSCAMERA_EXTENDEDPROP_EVCOMP_QUARTERSTEP` (1/4 EV per step unit).
pub const EV_COMP_FLAG_QUARTER_STEP: u64 = 2;
/// Windows `KSCAMERA_EXTENDEDPROP_EVCOMP_THIRDSTEP` (1/3 EV per step unit).
pub const EV_COMP_FLAG_THIRD_STEP: u64 = 4;
/// Windows `KSCAMERA_EXTENDEDPROP_EVCOMP_HALFSTEP` (1/2 EV per step unit).
pub const EV_COMP_FLAG_HALF_STEP: u64 = 8;
/// Windows `KSCAMERA_EXTENDEDPROP_EVCOMP_FULLSTEP` (1 EV per step unit).
pub const EV_COMP_FLAG_FULL_STEP: u64 = 16;

/// Map driver EV-comp stepping flags to EV stops per step unit.
#[must_use]
pub fn ev_comp_step_ev_from_flags(flags: u64) -> f32 {
    match flags {
        EV_COMP_FLAG_SIXTH_STEP => 1.0 / 6.0,
        EV_COMP_FLAG_QUARTER_STEP => 0.25,
        EV_COMP_FLAG_THIRD_STEP => 1.0 / 3.0,
        EV_COMP_FLAG_HALF_STEP => 0.5,
        EV_COMP_FLAG_FULL_STEP => 1.0,
        _ => 1.0 / 3.0,
    }
}

/// Convert a user-facing EV offset to driver step units (rounded to nearest step).
#[must_use]
pub fn user_ev_to_step_units(user_ev: f32, step_ev: f32) -> i32 {
    if step_ev <= f32::EPSILON {
        return 0;
    }
    (user_ev / step_ev).round() as i32
}

/// Clamp EV compensation step units to driver min/max.
#[must_use]
pub fn clamp_ev_comp_steps(value: i32, min: i32, max: i32) -> i32 {
    value.clamp(min, max)
}

/// Apply a relative hardware exposure step (manual fallback), clamped to range.
#[must_use]
pub fn apply_relative_exposure_step(
    last_applied: i32,
    delta_steps: i32,
    min: i32,
    max: i32,
) -> i32 {
    last_applied.saturating_add(delta_steps).clamp(min, max)
}

/// Map a fractional EV change to hardware integer steps (manual fallback).
#[must_use]
pub fn manual_ev_delta_to_hardware_steps(ev_delta: f32, hardware_step: i32) -> i32 {
    let hardware_step = hardware_step.max(1);
    (ev_delta * hardware_step as f32).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn third_step_driver_maps_half_ev_to_two_units() {
        let step_ev = ev_comp_step_ev_from_flags(EV_COMP_FLAG_THIRD_STEP);
        assert!((step_ev - (1.0 / 3.0)).abs() < f32::EPSILON);
        assert_eq!(user_ev_to_step_units(0.5, step_ev), 2);
    }

    #[test]
    fn half_step_driver_maps_half_ev_to_one_unit() {
        let step_ev = ev_comp_step_ev_from_flags(EV_COMP_FLAG_HALF_STEP);
        assert_eq!(user_ev_to_step_units(0.5, step_ev), 1);
    }

    #[test]
    fn zero_user_ev_yields_zero_step_units() {
        assert_eq!(user_ev_to_step_units(0.0, 1.0 / 3.0), 0);
    }

    #[test]
    fn clamp_ev_comp_respects_driver_range() {
        assert_eq!(clamp_ev_comp_steps(10, -6, 6), 6);
        assert_eq!(clamp_ev_comp_steps(-10, -6, 6), -6);
    }

    #[test]
    fn relative_exposure_step_clamps_at_bounds() {
        assert_eq!(apply_relative_exposure_step(-6, 2, -12, 0), -4);
        assert_eq!(apply_relative_exposure_step(0, 5, -12, 0), 0);
    }

    #[test]
    fn manual_ev_delta_rounds_to_nearest_hardware_step() {
        assert_eq!(manual_ev_delta_to_hardware_steps(0.5, 1), 1);
        assert_eq!(manual_ev_delta_to_hardware_steps(-0.5, 1), -1);
    }

    #[test]
    fn gain_bias_delta_steps_follow_ev_sign() {
        assert_eq!(apply_relative_exposure_step(64, 1, 0, 255), 65);
        assert_eq!(apply_relative_exposure_step(64, -1, 0, 255), 63);
    }
}
