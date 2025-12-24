pub mod errors;
pub mod signing;

use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use aptos_rest_client::{Client as AptosClient, PendingTransaction, Response, Transaction};
use aptos_sdk::{
    move_types::{identifier::Identifier, language_storage::ModuleId},
    types::{
        account_address::AccountAddress,
        transaction::{
            EntryFunction, RawTransaction,
            authenticator::{AccountAuthenticator, TransactionAuthenticator},
        },
    },
};
use privy_rs::PrivyClient;
pub use signing::{CapabilitySigner, SignedCapability};

use crate::{CONFIG, common::zresult::ZResult, zerror};

use aptos_sdk::crypto::ed25519::Ed25519Signature;
use aptos_sdk::types::{chain_id::ChainId, transaction::TransactionPayload};
use aptos_sdk::{crypto::ed25519::Ed25519PublicKey, types::transaction::SignedTransaction};
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
    let capability = generate_capability_for_publication(&data, 60)?;

    let response = mint_publish_capability(aptos, privy, &data, &capability).await?;
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
    let chain_id = aptos.get_index().await?.into_inner().chain_id;

    let publish_entry_function = EntryFunction::new(
        ModuleId::from_str("0x1::publish3::publication_registry")?,
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

    let user_wallet_pk = hex::decode(data.user_wallet_pk.trim_start_matches("0x"))
        .map_err(|err| zerror!("Failed to decode public key: {}", err))?;
    let publish_signature_hex =
        sign_with_privy(privy, &data.user_wallet_id, &publish_raw_txn).await?;
    let publish_authenticator = build_authenticator(&user_wallet_pk, &publish_signature_hex)?;
    let publish_signed_txn =
        SignedTransaction::new_signed_transaction(publish_raw_txn, publish_authenticator);

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
    let mint_capability_entry_function = EntryFunction::new(
        ModuleId::from_str("0x1::publish3::publication_registry")?,
        Identifier::new("mint_publish_capability_with_sig")?,
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
    let chain_id = aptos.get_index().await?.into_inner().chain_id;

    let expiration_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60;

    let mint_capability_raw_txn = RawTransaction::new(
        data.user_wallet,
        account.sequence_number,
        TransactionPayload::EntryFunction(mint_capability_entry_function),
        100_000,
        100,
        expiration_timestamp,
        ChainId::new(chain_id),
    );
    let user_wallet_pk = hex::decode(data.user_wallet_pk.trim_start_matches("0x"))
        .map_err(|err| zerror!("Failed to decode public key: {}", err))?;

    let mint_signature_hex =
        sign_with_privy(privy, &data.user_wallet_id, &mint_capability_raw_txn).await?;
    let mint_authenticator = build_authenticator(&user_wallet_pk, &mint_signature_hex)?;
    let mint_capability_signed_txn =
        SignedTransaction::new_signed_transaction(mint_capability_raw_txn, mint_authenticator);

    let mint_capability_transaction = aptos
        .submit(&mint_capability_signed_txn)
        .await?
        .into_inner();

    Ok(aptos
        .wait_for_transaction(&mint_capability_transaction)
        .await
        .map_err(|err| zerror!("Failed to mint publication capability: {}", err))?)
}

/// Create authenticator from public key and signature
fn build_authenticator(
    public_key: &Vec<u8>,
    signature_hex: &str,
) -> ZResult<TransactionAuthenticator> {
    let public_key = Ed25519PublicKey::try_from(public_key.as_slice())?;
    let signature_bytes = hex::decode(signature_hex)?;
    let signature = Ed25519Signature::try_from(signature_bytes.as_slice())?;
    let authenticator = AccountAuthenticator::ed25519(public_key, signature);

    Ok(TransactionAuthenticator::SingleSender {
        sender: authenticator,
    })
}

/// Sign a transaction using Privy wallet
async fn sign_with_privy(
    privy: &PrivyClient,
    wallet_id: &str,
    raw_txn: &RawTransaction,
) -> ZResult<String> {
    let signing_message = raw_txn.signing_message()?;
    let signing_message_hex = format!("0x{}", hex::encode(signing_message.clone()));

    let body = RawSign {
        params: RawSignParams::Variant0 {
            hash: signing_message_hex.clone(),
        },
    };

    let idempotency_key = format!("aptos-raw-sign:{}", hex::encode(signing_message));
    let ctx = AuthorizationContext::default();

    let response = privy
        .wallets()
        .raw_sign(wallet_id, &ctx, Some(&idempotency_key), &body)
        .await?;

    let signature_hex = response.data.signature.trim_start_matches("0x");
    Ok(signature_hex.to_string())
}
