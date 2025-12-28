use aptos_rust_sdk_types::error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl From<error::RestError> for BlockchainError {
    fn from(err: error::RestError) -> Self {
        BlockchainError::NetworkError(err.to_string())
    }
}

impl From<serde_json::Error> for BlockchainError {
    fn from(err: serde_json::Error) -> Self {
        BlockchainError::SerializationError(err.to_string())
    }
}
