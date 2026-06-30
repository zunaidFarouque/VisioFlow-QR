//! Live webcam decode resolution profiles — alternate full capture vs 720p downscale.

use image::DynamicImage;

use crate::optical::downsample;

/// Maximum width when decoding at 720p tier (1280×720 for 16:9).
pub const HD720_DECODE_MAX_WIDTH: u32 = 1280;

/// Which resolution tier to use for a live decode attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveDecodeProfile {
    /// Native capture resolution (e.g. 1080p).
    Full,
    /// Downscaled to ~720p width to reduce sensor noise.
    Hd720,
}

/// Alternate decode profiles: full resolution on even attempts, 720p on odd attempts.
pub fn alternating_live_decode_profile(attempt: u64) -> LiveDecodeProfile {
    if attempt.is_multiple_of(2) {
        LiveDecodeProfile::Full
    } else {
        LiveDecodeProfile::Hd720
    }
}

/// Prepare a frame for decode according to the active profile.
pub fn prepare_live_decode_frame(frame: &DynamicImage, profile: LiveDecodeProfile) -> DynamicImage {
    match profile {
        LiveDecodeProfile::Full => frame.clone(),
        LiveDecodeProfile::Hd720 => downsample(frame, HD720_DECODE_MAX_WIDTH),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, ImageBuffer, Rgba};

    #[test]
    fn alternating_profile_switches_every_attempt() {
        assert_eq!(alternating_live_decode_profile(0), LiveDecodeProfile::Full);
        assert_eq!(alternating_live_decode_profile(1), LiveDecodeProfile::Hd720);
        assert_eq!(alternating_live_decode_profile(2), LiveDecodeProfile::Full);
    }

    #[test]
    fn hd720_profile_downscales_1080p_frame() {
        let frame = DynamicImage::ImageRgba8(ImageBuffer::from_fn(1920, 1080, |_, _| {
            Rgba([0, 0, 0, 255])
        }));

        let prepared = prepare_live_decode_frame(&frame, LiveDecodeProfile::Hd720);
        assert_eq!(prepared.width(), 1280);
        assert_eq!(prepared.height(), 720);
    }

    #[test]
    fn full_profile_keeps_native_resolution() {
        let frame = DynamicImage::ImageRgba8(ImageBuffer::from_fn(1920, 1080, |_, _| {
            Rgba([0, 0, 0, 255])
        }));

        let prepared = prepare_live_decode_frame(&frame, LiveDecodeProfile::Full);
        assert_eq!(prepared.dimensions(), (1920, 1080));
    }
}
