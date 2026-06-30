#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_return)]
#![allow(clippy::collapsible_if)]

pub mod capture;
pub mod commands;
pub mod screen_bounds;
#[cfg(feature = "opencv-webcam")]
pub mod decode_worker;
#[cfg(feature = "opencv-webcam")]
pub mod preview_overlay;
#[cfg(feature = "opencv-webcam")]
pub mod webcam_preview;
#[cfg(feature = "opencv-webcam")]
pub mod webcam_session;
