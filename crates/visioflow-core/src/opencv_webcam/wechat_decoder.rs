#[cfg(feature = "opencv-webcam")]
use crate::error::{Result, VisioFlowError};
#[cfg(feature = "opencv-webcam")]
use crate::opencv_webcam::models::WeChatModelPaths;
#[cfg(feature = "opencv-webcam")]
use crate::traits::{BgrFrame, CnnQrDecoder};

#[cfg(feature = "opencv-webcam")]
use std::sync::Mutex;

#[cfg(feature = "opencv-webcam")]
pub struct WeChatCnnDecoder {
    inner: Mutex<opencv::wechat_qrcode::WeChatQRCode>,
}

#[cfg(feature = "opencv-webcam")]
impl WeChatCnnDecoder {
    pub fn init(model_paths: &WeChatModelPaths) -> Result<Self> {
        model_paths.validate()?;
        let detect_prototxt = model_paths.detect_prototxt.to_string_lossy();
        let detect_model = model_paths.detect_caffemodel.to_string_lossy();
        let sr_prototxt = model_paths.sr_prototxt.to_string_lossy();
        let sr_model = model_paths.sr_caffemodel.to_string_lossy();
        let inner = opencv::wechat_qrcode::WeChatQRCode::new(
            &detect_prototxt,
            &detect_model,
            &sr_prototxt,
            &sr_model,
        )
        .map_err(|error| {
            VisioFlowError::Decode(format!("failed to initialize WeChatQRCode models: {error}"))
        })?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }
}

#[cfg(feature = "opencv-webcam")]
impl CnnQrDecoder for WeChatCnnDecoder {
    fn decode_bgr(&self, frame: &BgrFrame) -> Result<Vec<String>> {
        use opencv::core::Mat;
        use opencv::prelude::{MatTraitManual, WeChatQRCodeTrait};

        let pixels = (frame.width as usize)
            .saturating_mul(frame.height as usize)
            .saturating_mul(3);
        if frame.data.len() < pixels {
            return Err(VisioFlowError::Decode(
                "BGR frame is smaller than expected dimensions".into(),
            ));
        }
        let mut mat = Mat::new_rows_cols_with_default(
            frame.height as i32,
            frame.width as i32,
            opencv::core::CV_8UC3,
            opencv::core::Scalar::default(),
        )
        .map_err(|error| VisioFlowError::Decode(format!("failed to allocate Mat: {error}")))?;
        let dst = mat.data_bytes_mut().map_err(|error| {
            VisioFlowError::Decode(format!("failed to access Mat bytes for decode: {error}"))
        })?;
        dst.copy_from_slice(&frame.data[..pixels]);

        let mut decoder = self.inner.lock().map_err(|_| {
            VisioFlowError::Decode("WeChat decoder mutex poisoned".into())
        })?;
        let payloads = decoder
            .detect_and_decode_def(&mat)
            .map_err(|error| VisioFlowError::Decode(format!("WeChat detect_and_decode failed: {error}")))?;
        let mut clean: Vec<String> = payloads
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        clean.dedup();
        if clean.is_empty() {
            return Err(VisioFlowError::NoPayloads);
        }
        Ok(clean)
    }
}
