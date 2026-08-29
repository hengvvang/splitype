use thiserror::Error;

/// Central result alias for splitype operations.
pub type CoreResult<T> = Result<T, CoreError>;

/// Foundational domain error enum.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Document error: {0}")]
    Document(String),

    #[error("Block not found: {0}")]
    BlockNotFound(u64),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

