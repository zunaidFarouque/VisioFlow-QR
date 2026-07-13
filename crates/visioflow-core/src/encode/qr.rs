use image::{GrayImage, Luma, Rgba, RgbaImage};
use qrcode::EcLevel;

use crate::error::{Result, VisioFlowError};

/// Error correction levels exposed to the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QrErrorCorrection {
    L,
    #[default]
    M,
    Q,
    H,
}

impl QrErrorCorrection {
    fn to_ec_level(self) -> EcLevel {
        match self {
            Self::L => EcLevel::L,
            Self::M => EcLevel::M,
            Self::Q => EcLevel::Q,
            Self::H => EcLevel::H,
        }
    }
}

/// Minimum quiet-zone margin in QR modules.
const QUIET_ZONE_MODULES: u32 = 4;

/// Result of encoding a payload into a QR bitmap.
#[derive(Debug, Clone)]
pub struct EncodedQr {
    pub image: RgbaImage,
    /// QR module width before quiet-zone pixel scaling (e.g. 21 for version 1).
    pub module_dimension: u32,
}

/// Encode `payload` into an RGBA QR image scaled to at least `min_pixel_size` per side.
pub fn encode_qr_rgba(
    payload: &str,
    ecc: QrErrorCorrection,
    min_pixel_size: u32,
) -> Result<EncodedQr> {
    let code = qrcode::QrCode::with_error_correction_level(payload.as_bytes(), ecc.to_ec_level())
        .map_err(|e| {
            let hint = if payload.len() > 800 {
                " — try shortening the text"
            } else {
                ""
            };
            VisioFlowError::Encode(format!(
                "payload too long for QR code ({} bytes){hint}: {e}",
                payload.len()
            ))
        })?;

    let dimension = code.width() as u32;
    let scale = (min_pixel_size / (dimension + 2 * QUIET_ZONE_MODULES))
        .max(4)
        .max(1);
    let modules_with_margin = dimension + 2 * QUIET_ZONE_MODULES;
    let image_size = modules_with_margin * scale;

    let modules = code.to_colors();
    let mut gray = GrayImage::from_pixel(image_size, image_size, Luma([255u8]));

    for y in 0..dimension {
        for x in 0..dimension {
            let idx = (y * dimension + x) as usize;
            if modules[idx] != qrcode::Color::Dark {
                continue;
            }
            let px = (x + QUIET_ZONE_MODULES) * scale;
            let py = (y + QUIET_ZONE_MODULES) * scale;
            for dy in 0..scale {
                for dx in 0..scale {
                    gray.put_pixel(px + dx, py + dy, Luma([0u8]));
                }
            }
        }
    }

    Ok(EncodedQr {
        image: gray_to_rgba(&gray),
        module_dimension: dimension,
    })
}

fn gray_to_rgba(gray: &GrayImage) -> RgbaImage {
    let (width, height) = gray.dimensions();
    let mut rgba = RgbaImage::new(width, height);
    for (x, y, pixel) in gray.enumerate_pixels() {
        let v = pixel.0[0];
        rgba.put_pixel(x, y, Rgba([v, v, v, 255]));
    }
    rgba
}

/// Render a grayscale QR image for decode tests and fixtures.
#[must_use]
pub fn encode_qr_gray(payload: &str, min_pixel_size: u32) -> GrayImage {
    let encoded = encode_qr_rgba(payload, QrErrorCorrection::M, min_pixel_size)
        .expect("valid test payload");
    let (w, h) = encoded.image.dimensions();
    let mut gray = GrayImage::new(w, h);
    for (x, y, pixel) in encoded.image.enumerate_pixels() {
        gray.put_pixel(x, y, Luma([pixel.0[0]]));
    }
    gray
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::RqrrDecoder;
    use crate::traits::PayloadDecoder;

    #[test]
    fn encodes_and_decodes_round_trip() {
        let payload = "https://visioflow.local/test";
        let image = encode_qr_gray(payload, 200);
        let decoder = RqrrDecoder;
        let decoded = decoder.decode(&image).expect("decode");
        assert_eq!(decoded, vec![payload]);
    }

    #[test]
    fn rejects_overlong_payload() {
        let payload = "x".repeat(10_000);
        let err = encode_qr_rgba(&payload, QrErrorCorrection::M, 256).expect_err("too long");
        assert!(matches!(err, VisioFlowError::Encode(msg) if msg.contains("too long")));
    }
}
