use std::path::{Path, PathBuf};

use crate::error::{Result, VisioFlowError};

const MODELS_ENV: &str = "VISIOFLOW_MODELS_DIR";

#[derive(Debug, Clone)]
pub struct WeChatModelPaths {
    pub detect_prototxt: PathBuf,
    pub detect_caffemodel: PathBuf,
    pub sr_prototxt: PathBuf,
    pub sr_caffemodel: PathBuf,
}

impl WeChatModelPaths {
    #[must_use]
    pub fn from_dir(models_dir: &Path) -> Self {
        Self {
            detect_prototxt: models_dir.join("detect.prototxt"),
            detect_caffemodel: models_dir.join("detect.caffemodel"),
            sr_prototxt: models_dir.join("sr.prototxt"),
            sr_caffemodel: models_dir.join("sr.caffemodel"),
        }
    }

    pub fn validate(&self) -> Result<()> {
        for path in [
            &self.detect_prototxt,
            &self.detect_caffemodel,
            &self.sr_prototxt,
            &self.sr_caffemodel,
        ] {
            if !path.exists() {
                return Err(VisioFlowError::Capture(format!(
                    "missing WeChat model file: {}",
                    path.display()
                )));
            }
        }
        Ok(())
    }
}

pub fn resolve_model_paths() -> Result<WeChatModelPaths> {
    let models_dir = resolve_models_dir()?;
    let paths = WeChatModelPaths::from_dir(&models_dir);
    paths.validate()?;
    Ok(paths)
}

fn resolve_models_dir() -> Result<PathBuf> {
    if let Ok(value) = std::env::var(MODELS_ENV) {
        let candidate = PathBuf::from(value);
        if candidate.exists() {
            return Ok(candidate);
        }
        return Err(VisioFlowError::Capture(format!(
            "{MODELS_ENV} is set but directory does not exist: {}",
            candidate.display()
        )));
    }

    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("models");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let exe = std::env::current_exe().map_err(|error| {
        VisioFlowError::Capture(format!(
            "failed to resolve current executable path: {error}"
        ))
    })?;
    for ancestor in exe.ancestors() {
        let candidate = ancestor.join("models");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(VisioFlowError::Capture(
        "unable to find /models directory; set VISIOFLOW_MODELS_DIR or run from project root"
            .into(),
    ))
}
