use std::collections::HashSet;

use image::DynamicImage;
use rxing::helpers::detect_in_image_filtered_with_hints;
use rxing::{BarcodeFormat, DecodeHints};

use crate::error::{Result, VisioFlowError};

/// Decode QR payloads using rxing (ZXing) — handles styled/circular-dot QR codes.
pub fn decode_with_rxing(frame: DynamicImage) -> Result<Vec<String>> {
    let mut hints = DecodeHints {
        TryHarder: Some(true),
        PossibleFormats: Some(HashSet::from([BarcodeFormat::QR_CODE])),
        ..DecodeHints::default()
    };

    let result =
        detect_in_image_filtered_with_hints(frame, Some(BarcodeFormat::QR_CODE), &mut hints)
            .map_err(|e| VisioFlowError::Decode(format!("rxing: {e}")))?;

    Ok(vec![result.getText().to_string()])
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
    fn decodes_styled_circular_dot_wifi_qr() {
        let path = styled_wifi_qr_path();
        if !path.exists() {
            return;
        }

        let image = image::open(&path).expect("open styled qr fixture");
        let payloads = decode_with_rxing(image).expect("rxing should decode styled qr");
        assert!(
            !payloads.is_empty(),
            "expected at least one payload from styled qr"
        );
        assert!(
            payloads[0].starts_with("WIFI:"),
            "wifi qr should decode to WIFI: payload, got {}",
            payloads[0]
        );
    }
}
