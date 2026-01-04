pub mod errors;
pub mod purchases;
pub mod signing;

use aptos_crypto::ed25519::{PublicKey, Signature};
use aptos_rust_sdk::client::rest_api::AptosFullnodeClient;
use aptos_rust_sdk_types::api_types::chain_id::ChainId;
use aptos_rust_sdk_types::api_types::transaction::{
    GenerateSigningMessage, SignedTransaction, TransactionPayload,
};
use aptos_rust_sdk_types::api_types::{
    module_id::ModuleId,
    transaction::{EntryFunction, RawTransaction},
};
use aptos_rust_sdk_types::state::State;
use privy_rs::{
    PrivateKey, PrivyClient,
    generated::{ResponseValue, types::RawSignResponse},
};
use serde::Serialize;
use serde_json::Value;

use crate::CAPABILITY_SIGNER;
use crate::{CONFIG, common::zresult::ZResult, zerror};
use aptos_rust_sdk_types::api_types::address::AccountAddress;
use aptos_rust_sdk_types::api_types::transaction_authenticator::TransactionAuthenticator;
use bcs;
use hex;
use privy_rs::{
    AuthorizationContext,
    generated::types::{RawSign, RawSignParams},
};
pub use signing::{CapabilitySigner, SignedCapability};
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize)]
pub struct TransactionSimulation {
    pub gas_used: u64,
    pub gas_unit_price: u64,
    pub total_cost_octas: u64,
    pub function: String,
}

#[derive(Debug, Serialize)]
pub struct SimulationSummary {
    pub mint_capability: TransactionSimulation,
    pub publish: TransactionSimulation,
    pub total_gas_cost_octas: u64,
}

pub struct PublicationData {
    pub paper_uid_hash: [u8; 32],
    pub paper_hash: [u8; 32],
    pub user_wallet: AccountAddress,
    pub user_wallet_id: String,
    pub user_wallet_pk: String,
    pub author_wallets: Vec<AccountAddress>,
    pub price: u64,
}

/// Submit a publication to the blockchain using the publish3 Move contract
pub async fn submit_publication_to_blockchain(
    aptos: &AptosFullnodeClient,
    privy: &PrivyClient,
    publication_data: PublicationData,
) -> ZResult<(Value, State)> {
    let capability =
        generate_capability_for_publication(&publication_data, 600).map_err(|err| {
            tracing::error!("Failed to generate capability for publication: {}", err);
            zerror!(err)
        })?;

    let (value, state) = mint_publish_capability(aptos, privy, &publication_data, &capability)
        .await
        .map_err(|err| {
            tracing::error!("Failed to mint publish capability: {}", err);
            zerror!(err)
        })?;

    tracing::debug!(
        "Minted publish capability. Value: {}. State: {:?}",
        value,
        state
    );

    submit_publish_transaction(aptos, privy, &publication_data).await
}

pub async fn simulate_publication_to_blockchain(
    aptos: &AptosFullnodeClient,
    privy: &PrivyClient,
    data: &PublicationData,
) -> ZResult<SimulationSummary> {
    let capability = generate_capability_for_publication(data, 600).map_err(|err| {
        tracing::error!("Failed to generate capability for publication: {}", err);
        zerror!(err)
    })?;

    let mint_publish_capability_txn =
        prepare_mint_capability_txn(aptos, privy, data, &capability, true).await?;
    let result = aptos
        .simulate_transaction(mint_publish_capability_txn)
        .await?;
    let mint_simulation_value = result.inner();
    tracing::debug!("Capability simulation result: {:?}", mint_simulation_value);

    let publish_signed_txn = prepare_publish_signed_txn(aptos, privy, data, true).await?;

    let publish_simulation_result = aptos.simulate_transaction(publish_signed_txn).await?;
    let publish_simulation_value = publish_simulation_result.inner();
    tracing::debug!("Publish simulation result: {:?}", publish_simulation_value);

    // Helper to parse a simulation value (array of one transaction simulation)
    let parse_simulation = |value: &Value, function_name: &str| -> ZResult<TransactionSimulation> {
        let arr = value
            .as_array()
            .ok_or_else(|| zerror!("Simulation result is not an array"))?;
        let first = arr
            .first()
            .ok_or_else(|| zerror!("Simulation result array is empty"))?;
        let gas_used = first
            .get("gas_used")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .parse::<u64>()
            .map_err(|e| zerror!("Failed to parse gas_used: {}", e))?;
        let gas_unit_price = first
            .get("gas_unit_price")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .parse::<u64>()
            .map_err(|e| zerror!("Failed to parse gas_unit_price: {}", e))?;
        let total_cost_octas = gas_used * gas_unit_price;
        Ok(TransactionSimulation {
            gas_used,
            gas_unit_price,
            total_cost_octas,
            function: function_name.to_string(),
        })
    };

    let mint_sim = parse_simulation(mint_simulation_value, "mint_publish_capability_with_sig")?;
    let publish_sim = parse_simulation(publish_simulation_value, "publish")?;
    let total_gas_cost_octas = mint_sim.total_cost_octas + publish_sim.total_cost_octas;

    Ok(SimulationSummary {
        mint_capability: mint_sim,
        publish: publish_sim,
        total_gas_cost_octas,
    })
}

async fn submit_publish_transaction(
    aptos: &AptosFullnodeClient,
    privy: &PrivyClient,
    data: &PublicationData,
) -> ZResult<(Value, State)> {
    let publish_signed_txn = prepare_publish_signed_txn(aptos, privy, data, false).await?;

    Ok(aptos
        .submit_transaction(publish_signed_txn)
        .await?
        .into_parts())
}

async fn prepare_publish_signed_txn(
    aptos: &AptosFullnodeClient,
    privy: &PrivyClient,
    data: &PublicationData,
    simulation: bool,
) -> ZResult<SignedTransaction> {
    let sequence_number = find_account_sequence_number(aptos, data.user_wallet.to_string()).await;

    let chain_id = 250;
    let module_id = ModuleId::new(
        AccountAddress::from_str(CONFIG.contract_address.as_str()).unwrap(),
        "publication_registry".to_string(),
    );

    let publish_entry_function = EntryFunction::new(
        module_id,
        "publish".to_string(),
        vec![],
        vec![bcs::to_bytes(&data.author_wallets)?],
    );

    let expiration_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 600;

    let publish_raw_txn = RawTransaction::new(
        data.user_wallet,
        sequence_number,
        TransactionPayload::EntryFunction(publish_entry_function),
        100_000,
        100,
        expiration_timestamp,
        ChainId::Other(chain_id),
    );

    let publish_authenticator = if simulation {
        build_simulation_authenticator(&data.user_wallet_pk)?
    } else {
        let sign_response = sign_with_privy(privy, &data.user_wallet_id, &publish_raw_txn).await?;
        build_authenticator(&data.user_wallet_pk, sign_response)?
    };

    let publish_signed_txn = SignedTransaction::new(publish_raw_txn, publish_authenticator);

    Ok(publish_signed_txn)
}

/// Generates a [SignedCapability] that allows the user's wallet to sign
/// the subsequent publication transaction from the smart contract.
///
/// Think of the signed capability as a token given to the client, allowing it to interact with
/// the smart contract.
fn generate_capability_for_publication(
    publication_data: &PublicationData,
    expiration_secs: u64,
) -> ZResult<SignedCapability> {
    let capability = CAPABILITY_SIGNER.create_capability(
        &publication_data.paper_uid_hash,
        &publication_data.paper_hash,
        publication_data.price,
        &publication_data.user_wallet,
        expiration_secs,
    )?;

    Ok(capability)
}

async fn mint_publish_capability(
    aptos: &AptosFullnodeClient,
    privy: &PrivyClient,
    data: &PublicationData,
    capability: &SignedCapability,
) -> ZResult<(Value, State)> {
    let mint_capability_signed_txn =
        prepare_mint_capability_txn(aptos, privy, data, capability, false).await?;

    tracing::debug!(
        "Submitting transaction: {:?}",
        mint_capability_signed_txn.raw_txn()
    );
    Ok(aptos
        .submit_transaction(mint_capability_signed_txn)
        .await?
        .into_parts())
}

async fn prepare_mint_capability_txn(
    aptos: &AptosFullnodeClient,
    privy: &PrivyClient,
    data: &PublicationData,
    capability: &SignedCapability,
    simulation: bool,
) -> ZResult<SignedTransaction> {
    let module_id = ModuleId::new(
        AccountAddress::from_str(CONFIG.contract_address.as_str()).unwrap(),
        "publication_registry".to_string(),
    );

    let mint_capability_entry_function = EntryFunction::new(
        module_id,
        "mint_publish_capability_with_sig".to_string(),
        vec![],
        vec![
            bcs::to_bytes(&capability.mint_payload.paper_uid_hash)?,
            bcs::to_bytes(&capability.mint_payload.paper_hash)?,
            bcs::to_bytes(&capability.mint_payload.price)?,
            bcs::to_bytes(&capability.mint_payload.recipient)?,
            bcs::to_bytes(&capability.mint_payload.expires_at)?,
            bcs::to_bytes(&capability.signature.to_bytes().to_vec())?,
        ],
    );

    let sequence_number = find_account_sequence_number(aptos, data.user_wallet.to_string()).await;

    let chain_id = 250;
    let expiration_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 600;

    let mint_capability_raw_txn = RawTransaction::new(
        data.user_wallet,
        sequence_number,
        TransactionPayload::EntryFunction(mint_capability_entry_function),
        100_000,
        100,
        expiration_timestamp,
        ChainId::from_u8(chain_id),
    );

    let mint_authenticator = if simulation {
        build_simulation_authenticator(&data.user_wallet_pk)?
    } else {
        let sign_response =
            sign_with_privy(privy, &data.user_wallet_id, &mint_capability_raw_txn).await?;
        build_authenticator(&data.user_wallet_pk, sign_response)?
    };

    let mint_capability_signed_txn =
        SignedTransaction::new(mint_capability_raw_txn, mint_authenticator);
    Ok(mint_capability_signed_txn)
}

pub(super) async fn find_account_sequence_number(
    aptos: &AptosFullnodeClient,
    address: String,
) -> u64 {
    match aptos.get_account_resources(address).await {
        Ok(response) => {
            let resource = response.into_inner();
            resource
                .iter()
                .find(|r| r.type_ == "0x1::account::Account")
                .and_then(|r| r.data.get("sequence_number"))
                .and_then(|v| v.as_str())
                .map(|s| s.parse::<u64>().unwrap_or(0))
                .unwrap_or(0)
        }
        Err(err) => {
            tracing::warn!("Failed to get account resources, assuming sequence number 0: {}", err);
            0
        }
    }
}

pub(super) fn build_authenticator(
    public_key_hex: &str,
    signature: ResponseValue<RawSignResponse>,
) -> ZResult<TransactionAuthenticator> {
    tracing::debug!("Building authenticator for public key: {}", public_key_hex);
    tracing::debug!("Raw signature from Privy: {}", signature.data.signature);

    let mut pk_bytes =
        hex::decode(public_key_hex.trim_start_matches("0x")).inspect_err(|&err| {
            tracing::error!("Hex decode error for public key: {}", err);
        })?;
    tracing::debug!("Public key bytes length: {}", pk_bytes.len());
    match pk_bytes.len() {
        32 => {} // correct
        33 if pk_bytes[0] == 0x00 => {
            pk_bytes.remove(0); // strip leading padding byte
            tracing::debug!("Stripped leading 0x00 byte from public key");
        }
        other => {
            return Err(zerror!(
                "Invalid Ed25519 public key length: {} bytes",
                other
            ));
        }
    }
    let sig_bytes =
        hex::decode(signature.data.signature.trim_start_matches("0x")).inspect_err(|&err| {
            tracing::error!("Hex decode error for signature: {}", err);
        })?;
    tracing::debug!("Signature bytes length: {}", sig_bytes.len());
    let sig = Signature::try_from(sig_bytes.as_slice()).map_err(|err| {
        tracing::error!("Error decoding signature: {}", err);
        err
    })?;
    let pk = PublicKey::try_from(pk_bytes.as_slice()).map_err(|err| {
        tracing::error!("Error decoding public key: {}", err);
        err
    })?;

    Ok(TransactionAuthenticator::ed25519(pk, sig))
}

pub(super) fn build_simulation_authenticator(
    public_key_hex: &str,
) -> ZResult<TransactionAuthenticator> {
    tracing::debug!(
        "Building simulation authenticator for public key: {}",
        public_key_hex
    );

    let mut pk_bytes =
        hex::decode(public_key_hex.trim_start_matches("0x")).inspect_err(|&err| {
            tracing::error!("Hex decode error for public key: {}", err);
        })?;
    tracing::debug!("Public key bytes length: {}", pk_bytes.len());
    match pk_bytes.len() {
        32 => {} // correct
        33 if pk_bytes[0] == 0x00 => {
            pk_bytes.remove(0); // strip leading padding byte
            tracing::debug!("Stripped leading 0x00 byte from public key");
        }
        other => {
            return Err(zerror!(
                "Invalid Ed25519 public key length: {} bytes",
                other
            ));
        }
    }
    let pk = PublicKey::try_from(pk_bytes.as_slice()).map_err(|err| {
        tracing::error!("Error decoding public key: {}", err);
        err
    })?;
    let zero_sig_bytes = [0u8; 64];
    let dummy_sig = Signature::try_from(&zero_sig_bytes[..]).map_err(|err| {
        tracing::error!("Failed to create dummy signature: {}", err);
        err
    })?;
    Ok(TransactionAuthenticator::ed25519(pk, dummy_sig))
}

pub(super) async fn sign_with_privy(
    privy: &PrivyClient,
    wallet_id: &str,
    raw_txn: &RawTransaction,
) -> ZResult<ResponseValue<RawSignResponse>> {
    let signing_message = raw_txn.generate_signing_message().unwrap();
    let message_hex = format!("0x{}", hex::encode(signing_message));

    tracing::debug!("MESSAGE HEX: {}", message_hex.clone());
    let body = RawSign {
        params: RawSignParams::Variant0 { hash: message_hex },
    };

    let ctx = AuthorizationContext::new().push(PrivateKey(CONFIG.privy_signer_key.to_owned()));

    privy
        .wallets()
        .raw_sign(wallet_id, &ctx, None, &body)
        .await
        .map_err(|e| zerror!("Privy signature failed: {:?}", e))
}
