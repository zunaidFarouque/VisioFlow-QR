use crate::decode::{decode_with_rxing, prepare_live_decode_frame, LiveDecodeProfile};
use crate::error::Result;
use crate::optical::{preprocess_frame, preprocess_frame_grayscale, MAX_FRAME_WIDTH};
use crate::traits::{FrameSource, OpticalFilterKind, PayloadDecoder};

/// Orchestrates frame capture, optical preprocessing, and payload decoding.
pub struct CaptureEngine<S, D> {
    source: S,
    decoder: D,
}

impl<S, D> CaptureEngine<S, D>
where
    S: FrameSource,
    D: PayloadDecoder,
{
    pub fn new(source: S, decoder: D) -> Self {
        Self { source, decoder }
    }

    pub fn run(&self, filter: OpticalFilterKind) -> Result<Vec<String>> {
        let frame = self.source.capture_frame()?;
        decode_captured_frame(&frame, filter, &self.decoder)
    }
}

/// Preprocess and decode a captured frame (rxing first, then binarized, then grayscale).
pub fn decode_captured_frame(
    frame: &image::DynamicImage,
    filter: OpticalFilterKind,
    decoder: &impl PayloadDecoder,
) -> Result<Vec<String>> {
    if let Ok(payloads) = decode_with_rxing(frame.clone()) {
        return Ok(payloads);
    }

    let binarized = preprocess_frame(frame, MAX_FRAME_WIDTH, filter);
    if let Ok(payloads) = decoder.decode(&binarized) {
        return Ok(payloads);
    }

    let grayscale = preprocess_frame_grayscale(frame, MAX_FRAME_WIDTH, filter);
    decoder.decode(&grayscale)
}

/// Live-capture decode: rxing first for styled QR, then rqrr fallbacks.
pub fn decode_captured_frame_live(
    frame: &image::DynamicImage,
    filter: OpticalFilterKind,
    decoder: &impl PayloadDecoder,
) -> Result<Vec<String>> {
    decode_captured_frame_live_with_profile(frame, filter, decoder, LiveDecodeProfile::Full)
}

/// Live-capture decode with an explicit resolution profile (full vs 720p downscale).
pub fn decode_captured_frame_live_with_profile(
    frame: &image::DynamicImage,
    filter: OpticalFilterKind,
    decoder: &impl PayloadDecoder,
    profile: LiveDecodeProfile,
) -> Result<Vec<String>> {
    let prepared = prepare_live_decode_frame(frame, profile);

    if let Ok(payloads) = decode_with_rxing(prepared.clone()) {
        return Ok(payloads);
    }

    let grayscale = preprocess_frame_grayscale(&prepared, MAX_FRAME_WIDTH, filter);
    if let Ok(payloads) = decoder.decode(&grayscale) {
        return Ok(payloads);
    }

    let binarized = preprocess_frame(&prepared, MAX_FRAME_WIDTH, filter);
    decoder.decode(&binarized)
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, GrayImage, Luma};

    use super::*;
    use crate::traits::MockFrameSource;
    use crate::traits::MockPayloadDecoder;

    fn solid_image(value: u8) -> DynamicImage {
        DynamicImage::ImageLuma8(GrayImage::from_pixel(32, 32, Luma([value])))
    }

    #[test]
    fn capture_engine_falls_back_to_grayscale_when_binarized_fails() {
        let mut source = MockFrameSource::new();
        source
            .expect_capture_frame()
            .returning(|| Ok(solid_image(128)));

        let mut decoder = MockPayloadDecoder::new();
        decoder
            .expect_decode()
            .times(1)
            .returning(|_| Err(crate::VisioFlowError::NoPayloads));
        decoder
            .expect_decode()
            .times(1)
            .returning(|_| Ok(vec!["fallback-payload".into()]));

        let engine = CaptureEngine::new(source, decoder);
        let payloads = engine.run(OpticalFilterKind::Otsu).expect("capture ok");
        assert_eq!(payloads, vec!["fallback-payload"]);
    }

    #[test]
    fn decode_captured_frame_live_tries_grayscale_before_binarized() {
        let image = solid_image(128);
        let mut decoder = MockPayloadDecoder::new();
        decoder
            .expect_decode()
            .times(1)
            .returning(|_| Ok(vec!["live-payload".into()]));

        let payloads =
            decode_captured_frame_live(&image, OpticalFilterKind::Otsu, &decoder).expect("decode");
        assert_eq!(payloads, vec!["live-payload"]);
    }

    #[test]
    fn capture_engine_runs_pipeline_and_decode() {
        let mut source = MockFrameSource::new();
        source
            .expect_capture_frame()
            .returning(|| Ok(solid_image(128)));

        let mut decoder = MockPayloadDecoder::new();
        decoder
            .expect_decode()
            .returning(|_| Ok(vec!["payload-a".into()]));

        let engine = CaptureEngine::new(source, decoder);
        let payloads = engine.run(OpticalFilterKind::Otsu).expect("capture ok");
        assert_eq!(payloads, vec!["payload-a"]);
    }
}
