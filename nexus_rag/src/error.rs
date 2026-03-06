use thiserror::Error;

#[derive(Debug, Error)]
pub enum NexusError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Qdrant error: {0}")]
    Qdrant(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Environment variable '{0}' not set or invalid")]
    EnvVar(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UUID parse error: {0}")]
    UuidParse(#[from] uuid::Error),

    #[error("Ungrounded response denied: {0}")]
    Ungrounded(String),

    #[error("Operation cancelled by operator")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, NexusError>;

pub fn qdrant_err<E: std::fmt::Display>(e: E) -> NexusError {
    NexusError::Qdrant(e.to_string())
}
