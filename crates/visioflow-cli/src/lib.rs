#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_return)]
#![allow(clippy::collapsible_if)]

pub mod capture;
pub mod clipboard_files;
pub mod clipboard_image;
pub mod commands;
#[cfg(feature = "opencv-webcam")]
pub mod decode_worker;
pub mod notifications;
pub mod preview_util;
pub mod qr_preview;
#[cfg(feature = "opencv-webcam")]
pub mod preview_overlay;
pub mod screen_bounds;
#[cfg(feature = "opencv-webcam")]
pub mod webcam_preview;
#[cfg(feature = "opencv-webcam")]
pub mod webcam_session;
