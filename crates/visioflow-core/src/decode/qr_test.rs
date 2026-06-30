use image::{GrayImage, Luma};

use crate::decode::RqrrDecoder;
use crate::traits::PayloadDecoder;

fn render_qr(payload: &str, size: u32) -> GrayImage {
    use qrcode::QrCode;

    let code = QrCode::new(payload.as_bytes()).expect("valid qr payload");
    let modules = code.to_colors();
    let dimension = code.width();
    let scale = (size / dimension as u32).max(4);
    let image_size = dimension as u32 * scale;

    let mut image = GrayImage::from_pixel(image_size, image_size, Luma([255u8]));
    for y in 0..dimension {
        for x in 0..dimension {
            let idx = y * dimension + x;
            if modules[idx] == qrcode::Color::Dark {
                for dy in 0..scale {
                    for dx in 0..scale {
                        image.put_pixel(x as u32 * scale + dx, y as u32 * scale + dy, Luma([0u8]));
                    }
                }
            }
        }
    }
    image
}

#[test]
fn decodes_single_qr_payload() {
    let image = render_qr("https://visioflow.local/test", 200);
    let decoder = RqrrDecoder;

    let payloads = decoder.decode(&image).expect("decode should succeed");
    assert_eq!(payloads, vec!["https://visioflow.local/test"]);
}

#[test]
fn returns_empty_when_no_qr_present() {
    let image = GrayImage::from_pixel(100, 100, Luma([128u8]));
    let decoder = RqrrDecoder;

    let result = decoder.decode(&image);
    assert!(result.is_err());
}
