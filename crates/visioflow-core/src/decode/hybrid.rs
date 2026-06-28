use image::{DynamicImage, GrayImage};

use crate::error::Result;
use crate::optical::{preprocess_frame, preprocess_frame_grayscale, MAX_FRAME_WIDTH};
use crate::traits::{OpticalFilterKind, PayloadDecoder};

use super::qr::RqrrDecoder;
use super::rxing::decode_with_rxing;

pub struct HybridQrDecoder;

impl PayloadDecoder for HybridQrDecoder {
    fn decode(&self, image: &GrayImage) -> Result<Vec<String>> {
        let frame = DynamicImage::ImageLuma8(image.clone());
        decode_dynamic_frame(&frame, OpticalFilterKind::Otsu)
    }
}

/// Try multiple decoders: rxing on color/grayscale first, then rqrr with preprocessing fallbacks.
pub fn decode_dynamic_frame(
    frame: &DynamicImage,
    filter: OpticalFilterKind,
) -> Result<Vec<String>> {
    if let Ok(payloads) = decode_with_rxing(frame.clone()) {
        return Ok(payloads);
    }

    let rqrr = RqrrDecoder;

    let grayscale = preprocess_frame_grayscale(frame, MAX_FRAME_WIDTH, filter);
    if let Ok(payloads) = rqrr.decode(&grayscale) {
        return Ok(payloads);
    }

    let binarized = preprocess_frame(frame, MAX_FRAME_WIDTH, filter);
    rqrr.decode(&binarized)
}

/// Live capture: rxing first (styled QR), then lightweight rqrr grayscale, then Otsu.
pub fn decode_dynamic_frame_live(
    frame: &DynamicImage,
    filter: OpticalFilterKind,
) -> Result<Vec<String>> {
    if let Ok(payloads) = decode_with_rxing(frame.clone()) {
        return Ok(payloads);
    }

    let rqrr = RqrrDecoder;

    let grayscale = preprocess_frame_grayscale(frame, MAX_FRAME_WIDTH, filter);
    if let Ok(payloads) = rqrr.decode(&grayscale) {
        return Ok(payloads);
    }

    let binarized = preprocess_frame(frame, MAX_FRAME_WIDTH, filter);
    rqrr.decode(&binarized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn styled_wifi_qr_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/c__Users_Zunaid_AppData_Roaming_Cursor_User_workspaceStorage_d4547ae2b0b71986b607de2e2e20b5b4_images_image-17a531fe-2327-4d30-8222-c26bdb3a82b6.png")
    }

    #[test]
    fn hybrid_decoder_decodes_styled_wifi_qr_from_file() {
        let path = styled_wifi_qr_path();
        if !path.exists() {
            return;
        }

        let image = image::open(&path).expect("open fixture");
        let payloads =
            decode_dynamic_frame_live(&image, OpticalFilterKind::Otsu).expect("decode styled qr");
        assert!(payloads[0].starts_with("WIFI:"));
    }
}
