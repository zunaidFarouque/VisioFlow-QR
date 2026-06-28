mod downsample;
mod otsu;
mod pipeline;
mod tone;

pub use downsample::{downsample, MAX_FRAME_WIDTH};
pub use otsu::{binarize_otsu, otsu_threshold};
pub use pipeline::{preprocess_frame, preprocess_frame_grayscale, run_optical_pipeline};
pub use tone::{apply_ev_adjustment, apply_ev_adjustment_f32};

#[cfg(test)]
mod downsample_test;
#[cfg(test)]
mod otsu_test;
#[cfg(test)]
mod pipeline_test;
