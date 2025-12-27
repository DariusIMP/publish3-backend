use aptos_crypto::ed25519::{PrivateKey, Signature};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_rust_sdk_types::api_types::address::AccountAddress;
use base64::{Engine, engine::general_purpose};

use crate::config::Config;

use super::errors::BlockchainError;

/// Represents a signed capability for publishing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignedCapability {
    pub mint_payload: MintPayload,
    pub signature: Signature, // Hex-encoded Ed25519 signature
}

/// MintPayload as defined in the Move contract
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct MintPayload {
    pub paper_hash: Vec<u8>,
    pub price: u64,
    pub recipient: AccountAddress,
    pub expires_at: u64,
}

pub struct CapabilitySigner {
    private_key: PrivateKey,
}

impl CapabilitySigner {
    pub fn from_config(config: &Config) -> Result<Self, BlockchainError> {
        let private_key_bytes = general_purpose::STANDARD
            .decode(&config.backend_private_key)
            .map_err(|e| BlockchainError::ConfigError(format!("Invalid private key base64: {}", e)))
            .or_else(|_| {
                hex::decode(&config.backend_private_key).map_err(|e| {
                    BlockchainError::ConfigError(format!("Invalid private key hex: {}", e))
                })
            })?;

        let private_key = PrivateKey::try_from(private_key_bytes.as_slice()).map_err(|e| {
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
            recipient: *recipient,
            expires_at,
        };

        let payload_bytes = aptos_bcs::to_bytes(&payload)
            .map_err(|e| BlockchainError::ConfigError(e.to_string()))?;

        let signature = self.private_key.sign_message(&payload_bytes);
        tracing::debug!(
            "Generated signature (hex): {}",
            hex::encode(signature.to_bytes())
        );

        Ok(SignedCapability {
            mint_payload: payload,
            signature,
        })
    }
}
