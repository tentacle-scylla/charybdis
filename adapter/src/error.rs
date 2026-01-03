//! Error types for Charybdis

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CharybdisError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("{0}")]
    Script(String),

    #[error("Benchmark cancelled")]
    Cancelled,

    #[error("Session already started")]
    AlreadyStarted,

    #[error("Latte error: {0}")]
    Latte(#[from] latte_core::error::LatteError),
}

pub type Result<T> = std::result::Result<T, CharybdisError>;
