pub mod bracket;
pub mod ev_comp;
pub mod hybrid;
pub mod live;
pub mod qr;
pub mod rxing;

pub use bracket::{clamp_exposure_value_f32, MANUAL_EV_STEP};
pub use ev_comp::{
    apply_relative_exposure_step, clamp_ev_comp_steps, ev_comp_step_ev_from_flags,
    manual_ev_delta_to_hardware_steps, user_ev_to_step_units,
};
pub use hybrid::{decode_dynamic_frame, decode_dynamic_frame_live, HybridQrDecoder};
pub use live::{
    alternating_live_decode_profile, prepare_live_decode_frame, LiveDecodeProfile,
    HD720_DECODE_MAX_WIDTH,
};
pub use qr::RqrrDecoder;
pub use rxing::decode_with_rxing;

#[cfg(test)]
mod qr_test;
