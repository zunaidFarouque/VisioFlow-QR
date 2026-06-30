use std::path::PathBuf;
use std::sync::Mutex;

use tempfile::tempdir;

use crate::opencv_webcam::models::{resolve_model_paths, WeChatModelPaths};

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        // SAFETY: tests hold ENV_LOCK so env mutations do not race.
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn touch_model_files(dir: &std::path::Path) {
    for name in [
        "detect.prototxt",
        "detect.caffemodel",
        "sr.prototxt",
        "sr.caffemodel",
    ] {
        std::fs::write(dir.join(name), b"x").expect("write model stub");
    }
}

#[test]
fn resolve_model_paths_uses_visioflow_models_dir_env() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let tmp = tempdir().expect("tempdir");
    let models = tmp.path().join("models");
    std::fs::create_dir_all(&models).expect("mkdir");
    touch_model_files(&models);

    let _guard = EnvGuard::set("VISIOFLOW_MODELS_DIR", models.to_str().expect("utf8 path"));
    let paths = resolve_model_paths().expect("resolve");
    assert_eq!(paths.detect_prototxt, models.join("detect.prototxt"));
}

#[test]
fn resolve_model_paths_errors_when_env_points_to_missing_dir() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let missing = PathBuf::from(r"C:\nonexistent-visioflow-models-dir");
    let _guard = EnvGuard::set("VISIOFLOW_MODELS_DIR", missing.to_str().expect("utf8 path"));
    let err = resolve_model_paths().expect_err("expected error");
    let message = err.to_string();
    assert!(message.contains("VISIOFLOW_MODELS_DIR"));
    assert!(message.contains("does not exist"));
}

#[test]
fn wechat_model_paths_validate_requires_all_files() {
    let tmp = tempdir().expect("tempdir");
    let models = tmp.path();
    std::fs::write(models.join("detect.prototxt"), b"x").expect("write");
    let paths = WeChatModelPaths::from_dir(models);
    let err = paths.validate().expect_err("expected missing files");
    assert!(err.to_string().contains("missing WeChat model file"));
}
