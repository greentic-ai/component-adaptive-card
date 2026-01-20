use thiserror::Error;

use crate::model::ValidationIssue;

#[derive(Debug, Error)]
pub enum ComponentError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("asset not found: {0}")]
    AssetNotFound(String),
    #[error("asset parse error: {0}")]
    AssetParse(String),
    #[error("asset error: {0}")]
    Asset(String),
    #[error("binding evaluation error: {0}")]
    Binding(String),
    #[error("card validation failed")]
    CardValidation(Vec<ValidationIssue>),
    #[error("interaction invalid: {0}")]
    InteractionInvalid(String),
    #[error("state store error: {0}")]
    StateStore(String),
}
