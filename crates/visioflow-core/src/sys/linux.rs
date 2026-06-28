use crate::error::{Result, VisioFlowError};

pub struct PlatformExecutor;

impl super::SystemExecutor for PlatformExecutor {
    fn connect_wifi(&self, _ssid: &str, _password: &str) -> Result<()> {
        Err(VisioFlowError::Capture(
            "wifi connection not implemented on linux".into(),
        ))
    }
}
