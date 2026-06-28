//! Webcam exposure: true EV compensation when supported, relative manual fallback otherwise.

use nokhwa::utils::{ControlValueDescription, KnownCameraControl};
#[cfg(not(target_os = "windows"))]
use nokhwa::utils::ControlValueSetter;
use nokhwa::Camera;
use visioflow_core::decode::{
    apply_relative_exposure_step, clamp_ev_comp_steps, ev_comp_step_ev_from_flags,
    manual_ev_delta_to_hardware_steps, user_ev_to_step_units,
};
use visioflow_core::error::{Result, VisioFlowError};

/// Frames to discard after changing sensor exposure so the ISP can settle.
pub const EXPOSURE_SETTLE_FRAMES: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExposureControlKind {
    EvComp,
    ManualRelative,
    None,
}

#[derive(Debug, Clone)]
struct ManualRelativeState {
    use_gain: bool,
    min: i32,
    max: i32,
    step: i32,
    last_applied: Option<i32>,
    in_manual: bool,
}

#[derive(Debug, Clone)]
struct EvCompState {
    min: i32,
    max: i32,
    step_ev: f32,
    last_steps: Option<i32>,
}

/// Arrow-key exposure controller (EV comp primary, manual relative fallback).
pub struct WebcamExposureController {
    kind: ExposureControlKind,
    ev_comp: Option<EvCompState>,
    manual: Option<ManualRelativeState>,
    previous_manual_ev: f32,
}

impl WebcamExposureController {
    #[must_use]
    pub fn probe(camera: &Camera) -> Self {
        #[cfg(target_os = "windows")]
        if let Some(probe) = camera.probe_ev_compensation() {
            let step_ev = ev_comp_step_ev_from_flags(probe.step_flags);
            return Self {
                kind: ExposureControlKind::EvComp,
                ev_comp: Some(EvCompState {
                    min: probe.min,
                    max: probe.max,
                    step_ev,
                    last_steps: None,
                }),
                manual: None,
                previous_manual_ev: 0.0,
            };
        }

        if let Some(manual) = Self::probe_manual_relative(camera) {
            return Self {
                kind: ExposureControlKind::ManualRelative,
                ev_comp: None,
                manual: Some(manual),
                previous_manual_ev: 0.0,
            };
        }

        Self {
            kind: ExposureControlKind::None,
            ev_comp: None,
            manual: None,
            previous_manual_ev: 0.0,
        }
    }

    fn probe_manual_relative(camera: &Camera) -> Option<ManualRelativeState> {
        if let Some((min, max, step, value)) =
            Self::probe_integer_control(camera, KnownCameraControl::Exposure)
        {
            return Some(ManualRelativeState {
                use_gain: false,
                min,
                max,
                step,
                last_applied: Some(value),
                in_manual: false,
            });
        }

        if let Some((min, max, step, value)) =
            Self::probe_integer_control(camera, KnownCameraControl::Gain)
        {
            return Some(ManualRelativeState {
                use_gain: true,
                min,
                max,
                step,
                last_applied: Some(value),
                in_manual: false,
            });
        }

        None
    }

    fn probe_integer_control(
        camera: &Camera,
        control: KnownCameraControl,
    ) -> Option<(i32, i32, i32, i32)> {
        let info = camera.camera_control(control).ok()?;
        match info.description() {
            ControlValueDescription::IntegerRange {
                min,
                max,
                value,
                step,
                ..
            } => Some((*min as i32, *max as i32, (*step as i32).max(1), *value as i32)),
            ControlValueDescription::Integer { value, step, .. } => {
                let baseline = *value as i32;
                let step = (*step as i32).max(1);
                let span = step * 12;
                Some((baseline - span, baseline + span, step, baseline))
            }
            _ => None,
        }
    }

    fn read_live_control_value(camera: &Camera, use_gain: bool) -> Result<i32> {
        let control = if use_gain {
            KnownCameraControl::Gain
        } else {
            KnownCameraControl::Exposure
        };
        let info = camera.camera_control(control).map_err(|e| {
            VisioFlowError::Capture(format!("failed to read live {control} value: {e}"))
        })?;
        match info.description() {
            ControlValueDescription::IntegerRange { value, .. }
            | ControlValueDescription::Integer { value, .. } => Ok(*value as i32),
            _ => Err(VisioFlowError::Capture(format!(
                "unsupported {control} value description"
            ))),
        }
    }

    #[must_use]
    pub fn is_supported(&self) -> bool {
        self.kind != ExposureControlKind::None
    }

    #[must_use]
    pub fn control_kind_label(&self) -> &'static str {
        match self.kind {
            ExposureControlKind::EvComp => "ev_comp",
            ExposureControlKind::ManualRelative => "manual_relative",
            ExposureControlKind::None => "none",
        }
    }

    /// Apply cumulative manual EV offset (↑ brighter, ↓ darker).
    pub fn apply_manual_ev(
        &mut self,
        camera: &mut Camera,
        manual_ev: f32,
        verbose: bool,
    ) -> Result<()> {
        if (manual_ev - self.previous_manual_ev).abs() < f32::EPSILON {
            return Ok(());
        }

        let delta_ev = manual_ev - self.previous_manual_ev;
        self.previous_manual_ev = manual_ev;

        match self.kind {
            ExposureControlKind::EvComp => self.apply_ev_comp(camera, manual_ev, verbose)?,
            ExposureControlKind::ManualRelative => {
                self.apply_manual_relative(camera, manual_ev, delta_ev, verbose)?;
            }
            ExposureControlKind::None => {}
        }

        Ok(())
    }

    fn apply_ev_comp(
        &mut self,
        camera: &mut Camera,
        manual_ev: f32,
        verbose: bool,
    ) -> Result<()> {
        let Some(state) = self.ev_comp.as_mut() else {
            return Ok(());
        };

        let steps = clamp_ev_comp_steps(
            user_ev_to_step_units(manual_ev, state.step_ev),
            state.min,
            state.max,
        );

        if state.last_steps == Some(steps) {
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        camera.set_ev_compensation_steps(steps).map_err(|e| {
            VisioFlowError::Capture(format!("failed to set EV compensation: {e}"))
        })?;

        state.last_steps = Some(steps);

        if verbose {
            eprintln!(
                "exposure: ev_comp {manual_ev:+.1} EV ({steps} step units, step={:.2} EV)",
                state.step_ev
            );
        }

        Ok(())
    }

    fn apply_manual_relative(
        &mut self,
        camera: &mut Camera,
        manual_ev: f32,
        delta_ev: f32,
        verbose: bool,
    ) -> Result<()> {
        let Some(state) = self.manual.as_mut() else {
            return Ok(());
        };

        if manual_ev == 0.0 {
            if state.in_manual {
                Self::restore_auto_exposure(camera)?;
                state.in_manual = false;
                state.last_applied = None;

                if verbose {
                    eprintln!("exposure: manual_relative restored auto exposure (0 EV)");
                }
            }
            return Ok(());
        }

        if !state.in_manual {
            let live = Self::read_live_control_value(camera, state.use_gain)?;
            Self::set_manual_control(camera, state.use_gain, live)?;
            state.last_applied = Some(live);
            state.in_manual = true;

            if verbose {
                eprintln!(
                    "exposure: manual_relative latched live {}={live} before adjustment",
                    if state.use_gain { "gain" } else { "shutter" }
                );
            }
        }

        let hw_delta = manual_ev_delta_to_hardware_steps(delta_ev, state.step);
        let last = state.last_applied.unwrap_or(0);
        let value = apply_relative_exposure_step(last, hw_delta, state.min, state.max);

        if state.last_applied == Some(value) {
            return Ok(());
        }

        Self::set_manual_control(camera, state.use_gain, value)?;
        state.last_applied = Some(value);

        if verbose {
            eprintln!(
                "exposure: manual_relative {manual_ev:+.1} EV ({})",
                if state.use_gain {
                    format!("gain={value}")
                } else {
                    format!("shutter={value}")
                }
            );
        }

        Ok(())
    }

    fn set_manual_control(camera: &mut Camera, use_gain: bool, value: i32) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            if use_gain {
                camera
                    .set_gain_manual(value)
                    .map_err(|e| VisioFlowError::Capture(format!("failed to set gain: {e}")))?;
            } else {
                camera.set_exposure_manual(value).map_err(|e| {
                    VisioFlowError::Capture(format!("failed to set manual exposure: {e}"))
                })?;
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let control = if use_gain {
                KnownCameraControl::Gain
            } else {
                KnownCameraControl::Exposure
            };
            camera
                .set_camera_control(control, ControlValueSetter::Integer(i64::from(value)))
                .map_err(|e| {
                    VisioFlowError::Capture(format!("failed to set {control}: {e}"))
                })?;
        }

        Ok(())
    }

    fn restore_auto_exposure(camera: &mut Camera) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            camera.restore_auto_exposure().map_err(|e| {
                VisioFlowError::Capture(format!("failed to restore auto exposure: {e}"))
            })?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = camera;
        }

        Ok(())
    }

    pub fn restore(&mut self, camera: &mut Camera) {
        match self.kind {
            ExposureControlKind::EvComp => {
                if let Some(state) = self.ev_comp.as_mut() {
                    if state.last_steps.is_some() {
                        #[cfg(target_os = "windows")]
                        let _ = camera.set_ev_compensation_steps(0);
                        state.last_steps = None;
                    }
                }
            }
            ExposureControlKind::ManualRelative => {
                if let Some(state) = self.manual.as_mut() {
                    if state.in_manual {
                        let _ = Self::restore_auto_exposure(camera);
                        state.in_manual = false;
                        state.last_applied = None;
                    }
                }
            }
            ExposureControlKind::None => {}
        }
        self.previous_manual_ev = 0.0;
    }
}

pub fn log_exposure_status(controller: &WebcamExposureController, verbose: bool) {
    if !verbose {
        return;
    }

    match controller.kind {
        ExposureControlKind::EvComp => {
            if let Some(state) = controller.ev_comp.as_ref() {
                eprintln!(
                    "exposure: EV compensation available (range {}..={}, step {:.2} EV)",
                    state.min, state.max, state.step_ev
                );
            }
        }
        ExposureControlKind::ManualRelative => {
            if let Some(state) = controller.manual.as_ref() {
                let label = if state.use_gain { "gain" } else { "shutter" };
                eprintln!(
                    "exposure: manual relative fallback via {label} (range {}..={})",
                    state.min, state.max
                );
            }
        }
        ExposureControlKind::None => {
            eprintln!(
                "exposure: WARNING — no exposure controls; arrow keys cannot change capture brightness"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_controller_has_no_kind() {
        let controller = WebcamExposureController {
            kind: ExposureControlKind::None,
            ev_comp: None,
            manual: None,
            previous_manual_ev: 0.0,
        };
        assert!(!controller.is_supported());
        assert_eq!(controller.control_kind_label(), "none");
    }
}
