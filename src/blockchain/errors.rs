use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("Movement SDK error: {0}")]
    SdkError(String),

    #[error("Transaction failed: {0}")]
    TransactionError(String),

    #[error("Contract interaction error: {0}")]
    ContractError(String),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl From<aptos_sdk::rest_client::error::RestError> for BlockchainError {
    fn from(err: aptos_sdk::rest_client::error::RestError) -> Self {
        BlockchainError::NetworkError(err.to_string())
    }
}

impl From<serde_json::Error> for BlockchainError {
    fn from(err: serde_json::Error) -> Self {
        BlockchainError::SerializationError(err.to_string())
    }
}
