use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuleError {
    #[error("rule not found: {0}")]
    NotFound(String),

    #[error("regex did not match payload")]
    NoMatch,

    #[error("invalid regex: {0}")]
    InvalidRegex(String),

    #[error("config directory unavailable")]
    ConfigDirUnavailable,

    #[error("store io error: {0}")]
    StoreIo(String),

    #[error("store parse error: {0}")]
    StoreParse(String),

    #[error("exec failed: {0}")]
    ExecFailed(String),

    #[error("wifi connect failed: {0}")]
    WifiConnectFailed(String),
}

pub type Result<T> = std::result::Result<T, RuleError>;
