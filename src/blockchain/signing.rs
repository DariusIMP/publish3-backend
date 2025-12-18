use aptos_crypto::{SigningKey, ed25519::Ed25519PrivateKey};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_sdk::types::account_address::AccountAddress;

use crate::config::Config;

use super::errors::BlockchainError;

/// Represents a signed capability for publishing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignedCapability {
    pub paper_hash: String, // Hex-encoded paper hash
    pub price: u64,         // Paper price
    pub recipient: String,  // Recipient address (hex)
    pub expires_at: u64,    // Unix timestamp
    pub signature: String,  // Hex-encoded Ed25519 signature
}

/// MintPayload as defined in the Move contract
#[derive(serde::Serialize, serde::Deserialize, CryptoHasher, BCSCryptoHash)]
struct MintPayload {
    paper_hash: Vec<u8>,
    price: u64,
    recipient: Vec<u8>,
    expires_at: u64,
}

pub struct CapabilitySigner {
    private_key: Ed25519PrivateKey,
}

impl CapabilitySigner {
    pub fn from_config(config: &Config) -> Result<Self, BlockchainError> {
        let private_key_bytes = base64::decode(&config.backend_private_key)
            .map_err(|e| BlockchainError::ConfigError(format!("Invalid private key base64: {}", e)))
            .or_else(|_| {
                hex::decode(&config.backend_private_key).map_err(|e| {
                    BlockchainError::ConfigError(format!("Invalid private key hex: {}", e))
                })
            })?;

        let private_key =
            Ed25519PrivateKey::try_from(private_key_bytes.as_slice()).map_err(|e| {
                BlockchainError::ConfigError(format!("Invalid Ed25519 private key: {}", e))
            })?;

        Ok(Self { private_key })
    }

    pub fn create_capability(
        &self,
        paper_hash: &[u8],
        price: u64,
        recipient: &AccountAddress,
        expires_in_seconds: u64,
    ) -> Result<SignedCapability, BlockchainError> {
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| BlockchainError::ConfigError(format!("Time error: {}", e)))?
            .as_secs()
            + expires_in_seconds;

        let payload = MintPayload {
            paper_hash: paper_hash.to_vec(),
            price,
            recipient: recipient.to_vec(),
            expires_at,
        };

        let signature = self.private_key.sign(&payload).unwrap();

        Ok(SignedCapability {
            paper_hash: hex::encode(paper_hash),
            price,
            recipient: hex::encode(recipient.to_vec()),
            expires_at,
            signature: hex::encode(signature.to_bytes()),
        })
    }
}
