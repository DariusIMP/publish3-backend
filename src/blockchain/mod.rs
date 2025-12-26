pub mod errors;
pub mod signing;

use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use aptos_rest_client::{Client as AptosClient, PendingTransaction, Response, Transaction};
use aptos_sdk::{
    move_types::{identifier::Identifier, language_storage::ModuleId},
    types::{
        account_address::AccountAddress,
        transaction::{
            EntryFunction, RawTransaction,
            authenticator::{AccountAuthenticator, AuthenticationKey, TransactionAuthenticator},
        },
    },
};
use privy_rs::{
    PrivateKey, PrivyClient,
    generated::{ResponseValue, types::RawSignResponse},
};
use sha3::{Digest, Sha3_256};
pub use signing::{CapabilitySigner, SignedCapability};
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{CONFIG, common::zresult::ZResult, zerror};

use aptos_sdk::types::transaction::SignedTransaction;
use aptos_sdk::types::{chain_id::ChainId, transaction::TransactionPayload};
use bcs;
use hex;
use privy_rs::{
    AuthorizationContext,
    generated::types::{RawSign, RawSignParams},
};

pub struct PublicationData {
    pub paper_hash: [u8; 32],
    pub user_wallet: AccountAddress,
    pub user_wallet_id: String,
    pub user_wallet_pk: String,
    pub author_wallets: Vec<AccountAddress>,
    pub price: u64,
}

/// Submit a publication to the blockchain using the publish3 Move contract
pub async fn submit_publication_to_blockchain(
    aptos: &AptosClient,
    privy: &PrivyClient,
    data: PublicationData,
) -> ZResult<PendingTransaction> {
    let capability = generate_capability_for_publication(&data, 60).map_err(|err| {
        tracing::error!("Failed to generate capability for publication: {}", err);
        zerror!(err)
    })?;

    let response = mint_publish_capability(aptos, privy, &data, &capability)
        .await
        .map_err(|err| {
            tracing::error!("Failed to mint publish capability: {}", err);
            zerror!(err)
        })?;
    if !response.inner().success() {
        return Err(zerror!("Publication to blockchain failed."));
    }

    let transaction_response = submit_publish_transaction(aptos, privy, &data).await?;
    Ok(transaction_response)
}

async fn submit_publish_transaction(
    aptos: &AptosClient,
    privy: &PrivyClient,
    data: &PublicationData,
) -> ZResult<PendingTransaction> {
    let account = aptos.get_account(data.user_wallet).await?.into_inner();
    let chain_id = 250;
    let module_id =
        ModuleId::from_str(format!("{}::publication_registry", CONFIG.contract_address).as_str())
            .map_err(|err| {
            tracing::error!("Failed to parse module id: {}", err);
            err
        })?;

    let publish_entry_function = EntryFunction::new(
        module_id,
        Identifier::new("publish")?,
        vec![],
        vec![bcs::to_bytes(&data.author_wallets)?],
    );

    let expiration_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 600;

    let publish_raw_txn = RawTransaction::new(
        data.user_wallet,
        account.sequence_number,
        TransactionPayload::EntryFunction(publish_entry_function),
        100_000,
        100,
        expiration_timestamp,
        ChainId::new(chain_id),
    );

    let sign_response = sign_with_privy(privy, &data.user_wallet_id, &publish_raw_txn).await?;

    let publish_authenticator = build_authenticator(&data.user_wallet_pk, sign_response)?;
    let publish_signed_txn = SignedTransaction::new_signed_transaction(
        publish_raw_txn,
        TransactionAuthenticator::SingleSender {
            sender: publish_authenticator,
        },
    );

    let pending = aptos.submit(&publish_signed_txn).await?.into_inner();
    Ok(pending)
}

/// This backend server generates a [SignedCapability] that allows the user's wallet to sign
/// the subsequent publication transaction from the smart contract.
///
/// Think of the signed capability as a token given to the client, allowing it to interact with
/// the smart contract.
fn generate_capability_for_publication(
    publication_data: &PublicationData,
    expiration_secs: u64,
) -> ZResult<SignedCapability> {
    let capability_signer = CapabilitySigner::from_config(&CONFIG)
        .map_err(|err| zerror!("Failed to create capability signer: {}", err))?;

    let capability = capability_signer.create_capability(
        &publication_data.paper_hash,
        publication_data.price,
        &publication_data.user_wallet,
        expiration_secs,
    )?;

    Ok(capability)
}

async fn mint_publish_capability(
    aptos: &AptosClient,
    privy: &PrivyClient,
    data: &PublicationData,
    capability: &SignedCapability,
) -> ZResult<Response<Transaction>> {
    let identifier = Identifier::new("mint_publish_capability_with_sig")?;
    let module_id =
        ModuleId::from_str(format!("{}::publication_registry", CONFIG.contract_address).as_str())?;

    let mint_capability_entry_function = EntryFunction::new(
        module_id,
        identifier,
        vec![],
        vec![
            bcs::to_bytes(&data.paper_hash)?,
            bcs::to_bytes(&data.price)?,
            bcs::to_bytes(&data.user_wallet)?,
            bcs::to_bytes(&capability.expires_at)?,
            bcs::to_bytes(&capability.signature)?,
        ],
    );

    let account = aptos.get_account(data.user_wallet).await?.into_inner();

    let chain_id = 250;
    let expiration_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 600;

    let mint_capability_raw_txn = RawTransaction::new(
        data.user_wallet,
        account.sequence_number,
        TransactionPayload::EntryFunction(mint_capability_entry_function),
        100_000,
        1,
        expiration_timestamp,
        ChainId::new(chain_id),
    );

    tracing::debug!("Raw txn: {:?}", mint_capability_raw_txn);

    let signing_message_before = mint_capability_raw_txn.signing_message()?;
    tracing::debug!(
        "SIGNING_MESSAGE_BEFORE (hex): {}",
        hex::encode(&signing_message_before)
    );

    let sign_response =
        sign_with_privy(privy, &data.user_wallet_id, &mint_capability_raw_txn).await?;

    let mint_authenticator = build_authenticator(&data.user_wallet_pk, sign_response)?;

    let mint_capability_signed_txn = SignedTransaction::new_signed_transaction(
        mint_capability_raw_txn,
        TransactionAuthenticator::SingleSender {
            sender: mint_authenticator,
        },
    );

    tracing::debug!("Signed txn: {:?}", mint_capability_signed_txn);

    tracing::debug!("Submitting transaction...");

    let pending_txn = aptos
        .submit(&mint_capability_signed_txn)
        .await?
        .into_inner();

    Ok(aptos.wait_for_transaction(&pending_txn).await?)
}

fn build_authenticator(
    public_key_hex: &str,
    signature: ResponseValue<RawSignResponse>,
) -> ZResult<AccountAuthenticator> {
    let mut pk_bytes = hex::decode(public_key_hex.trim_start_matches("0x")).map_err(|err| {
        tracing::error!("Hex decode error for public key: {}", err);
        err
    })?;
    match pk_bytes.len() {
        32 => {} // correct
        33 if pk_bytes[0] == 0x00 => {
            pk_bytes.remove(0); // strip leading padding byte
        }
        other => {
            return Err(zerror!(
                "Invalid Ed25519 public key length: {} bytes",
                other
            ));
        }
    }
    let sig_bytes =
        hex::decode(signature.data.signature.trim_start_matches("0x")).map_err(|err| {
            tracing::error!("Hex decode error for signature: {}", err);
            err
        })?;
    let sig = Ed25519Signature::try_from(sig_bytes.as_slice()).map_err(|err| {
        tracing::error!("Error decoding signature: {}", err);
        err
    })?;
    let pk = Ed25519PublicKey::try_from(pk_bytes.as_slice()).map_err(|err| {
        tracing::error!("Error decoding public key: {}", err);
        err
    })?;

    Ok(AccountAuthenticator::Ed25519 {
        public_key: pk,
        signature: sig,
    })
}

async fn sign_with_privy(
    privy: &PrivyClient,
    wallet_id: &str,
    raw_txn: &RawTransaction,
) -> ZResult<ResponseValue<RawSignResponse>> {
    let txn_bytes = bcs::to_bytes(raw_txn).map_err(|e| zerror!("BCS encode failed: {}", e))?;

    let mut to_hash = b"APTOS::RawTransaction".to_vec();
    to_hash.extend(txn_bytes);

    let digest = Sha3_256::digest(&to_hash);

    let message_hex = format!("0x{}", hex::encode(digest));

    let body = RawSign {
        params: RawSignParams::Variant0 {
            hash: message_hex.clone(),
        },
    };

    let idempotency_key = format!("aptos-raw-sign:{}", message_hex);

    let ctx = AuthorizationContext::new().push(PrivateKey(CONFIG.privy_signer_key.to_owned()));

    Ok(privy
        .wallets()
        .raw_sign(wallet_id, &ctx, Some(&idempotency_key), &body)
        .await
        .map_err(|e| zerror!("Privy signature failed: {}", e))?)
}
