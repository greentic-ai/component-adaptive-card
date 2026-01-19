use thiserror::Error;

#[derive(Debug, Error)]
pub enum ComponentError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("asset error: {0}")]
    Asset(String),
    #[error("state store error: {0}")]
    StateStore(String),
}
