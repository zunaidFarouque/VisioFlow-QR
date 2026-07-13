pub mod internet_shortcut;
pub mod normalize;
pub mod qr;

pub use internet_shortcut::{
    is_internet_shortcut_path, parse_internet_shortcut_contents, parse_internet_shortcut_file,
};
pub use normalize::resolve_encode_payload;
pub use qr::{encode_qr_gray, encode_qr_rgba, EncodedQr, QrErrorCorrection};
