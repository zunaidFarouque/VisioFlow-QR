//! Webcam exposure: true EV compensation when supported, gain-bias hardware fallback otherwise.

use nokhwa::utils::{ControlValueDescription, KnownCameraControl};
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
    GainBias,
    None,
}

#[derive(Debug, Clone)]
struct GainBiasState {
    min: i32,
    max: i32,
    step: i32,
    last_applied: Option<i32>,
    in_manual_gain: bool,
}

#[derive(Debug, Clone)]
struct EvCompState {
    min: i32,
    max: i32,
    step_ev: f32,
    last_steps: Option<i32>,
}

/// Arrow-key exposure controller (EV comp primary, gain-bias fallback).
pub struct WebcamExposureController {
    kind: ExposureControlKind,
    ev_comp: Option<EvCompState>,
    gain_bias: Option<GainBiasState>,
    previous_manual_ev: f32,
}

impl WebcamExposureController {
    #[must_use]
    pub fn probe(camera: &Camera, verbose: bool) -> Self {
        if verbose {
            log_ev_comp_probe_diagnostics(camera);
        }

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
                gain_bias: None,
                previous_manual_ev: 0.0,
            };
        }

        if let Some(gain_bias) = Self::probe_gain_bias(camera) {
            return Self {
                kind: ExposureControlKind::GainBias,
                ev_comp: None,
                gain_bias: Some(gain_bias),
                previous_manual_ev: 0.0,
            };
        }

        Self {
            kind: ExposureControlKind::None,
            ev_comp: None,
            gain_bias: None,
            previous_manual_ev: 0.0,
        }
    }

    fn probe_gain_bias(camera: &Camera) -> Option<GainBiasState> {
        let (min, max, step, _value) =
            Self::probe_integer_control(camera, KnownCameraControl::Gain)?;
        Some(GainBiasState {
            min,
            max,
            step,
            last_applied: None,
            in_manual_gain: false,
        })
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

    fn read_live_gain(camera: &Camera) -> Result<i32> {
        let info = camera.camera_control(KnownCameraControl::Gain).map_err(|e| {
            VisioFlowError::Capture(format!("failed to read live gain value: {e}"))
        })?;
        match info.description() {
            ControlValueDescription::IntegerRange { value, .. }
            | ControlValueDescription::Integer { value, .. } => Ok(*value as i32),
            _ => Err(VisioFlowError::Capture(
                "unsupported gain value description".into(),
            )),
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
            ExposureControlKind::GainBias => "gain_bias",
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
            ExposureControlKind::GainBias => {
                self.apply_gain_bias(camera, manual_ev, delta_ev, verbose)?;
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

    fn apply_gain_bias(
        &mut self,
        camera: &mut Camera,
        manual_ev: f32,
        delta_ev: f32,
        verbose: bool,
    ) -> Result<()> {
        let Some(state) = self.gain_bias.as_mut() else {
            return Ok(());
        };

        if manual_ev == 0.0 {
            if state.in_manual_gain {
                Self::restore_auto_gain(camera)?;
                state.in_manual_gain = false;
                state.last_applied = None;

                if verbose {
                    eprintln!("exposure: gain_bias restored auto gain (0 EV, exposure stays auto)");
                }
            }
            return Ok(());
        }

        if !state.in_manual_gain {
            let live = Self::read_live_gain(camera)?;
            Self::set_gain_manual(camera, live)?;
            state.last_applied = Some(live);
            state.in_manual_gain = true;

            if verbose {
                eprintln!("exposure: gain_bias latched live gain={live} (exposure stays auto)");
            }
        }

        let hw_delta = manual_ev_delta_to_hardware_steps(delta_ev, state.step);
        let last = state.last_applied.unwrap_or(0);
        let value = apply_relative_exposure_step(last, hw_delta, state.min, state.max);

        if state.last_applied == Some(value) {
            return Ok(());
        }

        Self::set_gain_manual(camera, value)?;
        state.last_applied = Some(value);

        if verbose {
            eprintln!("exposure: gain_bias {manual_ev:+.1} EV (gain={value}, exposure auto)");
        }

        Ok(())
    }

    fn set_gain_manual(camera: &mut Camera, value: i32) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            camera
                .set_gain_manual(value)
                .map_err(|e| VisioFlowError::Capture(format!("failed to set gain: {e}")))?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (camera, value);
        }

        Ok(())
    }

    fn restore_auto_gain(camera: &mut Camera) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            camera.restore_auto_gain().map_err(|e| {
                VisioFlowError::Capture(format!("failed to restore auto gain: {e}"))
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
            ExposureControlKind::GainBias => {
                if let Some(state) = self.gain_bias.as_mut() {
                    if state.in_manual_gain {
                        let _ = Self::restore_auto_gain(camera);
                        state.in_manual_gain = false;
                        state.last_applied = None;
                    }
                }
            }
            ExposureControlKind::None => {}
        }
        self.previous_manual_ev = 0.0;
    }
}

#[cfg(target_os = "windows")]
fn log_ev_comp_probe_diagnostics(camera: &Camera) {
    let diagnostic = camera.probe_ev_compensation_diagnostic();
    let media_source = diagnostic
        .media_source_controller
        .as_deref()
        .unwrap_or("not tried");
    let source_reader = diagnostic
        .source_reader_controller
        .as_deref()
        .unwrap_or("not tried");
    let extended = diagnostic
        .get_extended_control
        .as_deref()
        .unwrap_or("not tried");
    let lock = diagnostic
        .lock_payload
        .as_deref()
        .unwrap_or("not tried");

    eprintln!("exposure: probe ev_comp via IMFMediaSource … {media_source}");
    eprintln!("exposure: probe ev_comp via IMFSourceReader … {source_reader}");
    eprintln!("exposure: probe ev_comp GetExtendedCameraControl … {extended}");
    eprintln!("exposure: probe ev_comp LockPayload … {lock}");

    if let Some(probe) = diagnostic.probe {
        let step_ev = ev_comp_step_ev_from_flags(probe.step_flags);
        eprintln!(
            "exposure: probe ev_comp OK (range {}..={}, step {step_ev:.2} EV)",
            probe.min, probe.max
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn log_ev_comp_probe_diagnostics(_camera: &Camera) {}

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
        ExposureControlKind::GainBias => {
            if let Some(state) = controller.gain_bias.as_ref() {
                eprintln!(
                    "exposure: gain_bias fallback (gain range {}..={}, exposure stays auto)",
                    state.min, state.max
                );
            }
        }
        ExposureControlKind::None => {
            eprintln!(
                "exposure: WARNING — no hardware EV comp or gain control; arrow keys disabled"
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
            gain_bias: None,
            previous_manual_ev: 0.0,
        };
        assert!(!controller.is_supported());
        assert_eq!(controller.control_kind_label(), "none");
    }

    #[test]
    fn gain_bias_label_differs_from_manual_relative() {
        let controller = WebcamExposureController {
            kind: ExposureControlKind::GainBias,
            ev_comp: None,
            gain_bias: Some(GainBiasState {
                min: 0,
                max: 255,
                step: 1,
                last_applied: None,
                in_manual_gain: false,
            }),
            previous_manual_ev: 0.0,
        };
        assert_eq!(controller.control_kind_label(), "gain_bias");
    }
}
