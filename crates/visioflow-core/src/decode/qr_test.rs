use crate::decode::RqrrDecoder;
use crate::encode::encode_qr_gray;
use crate::traits::PayloadDecoder;

#[test]
fn decodes_single_qr_payload() {
    let image = encode_qr_gray("https://visioflow.local/test", 200);
    let decoder = RqrrDecoder;

    let payloads = decoder.decode(&image).expect("decode should succeed");
    assert_eq!(payloads, vec!["https://visioflow.local/test"]);
}

#[test]
fn returns_empty_when_no_qr_present() {
    use image::{GrayImage, Luma};

    let image = GrayImage::from_pixel(100, 100, Luma([128u8]));
    let decoder = RqrrDecoder;

    let result = decoder.decode(&image);
    assert!(result.is_err());
}
