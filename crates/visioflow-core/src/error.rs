use thiserror::Error;

#[derive(Debug, Error)]
pub enum VisioFlowError {
    #[error("capture failed: {0}")]
    Capture(String),

    #[error("optical processing failed: {0}")]
    Optical(String),

    #[error("decode failed: {0}")]
    Decode(String),

    #[error("no payloads found in image")]
    NoPayloads,

    #[error("unsupported action: {0}")]
    UnsupportedAction(String),

    #[error("ipc error: {0}")]
    Ipc(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error(
        "air-gap mode: refusing to start; network telemetry (OTLP) is not permitted. \
         Unset VISIOFLOW_AIRGAP or omit --disable-telemetry."
    )]
    AirGap,
}

pub type Result<T> = std::result::Result<T, VisioFlowError>;
