use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("cloudflared exe path not set")]
    ExeNotSet,
    #[error("cloudflared exe not found at: {0}")]
    ExeNotFound(String),
    #[error("cloudflared command failed: {0}")]
    CommandFailed(String),
    #[error("not logged in (cert.pem missing) — run login first")]
    NotLoggedIn,
    #[error("tunnel {0} is already running")]
    AlreadyRunning(String),
    #[error("tunnel {0} is not running")]
    NotRunning(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),
    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
