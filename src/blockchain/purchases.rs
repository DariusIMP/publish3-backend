use aptos_rust_sdk::client::rest_api::AptosFullnodeClient;
use aptos_rust_sdk_types::api_types::chain_id::ChainId;
use aptos_rust_sdk_types::api_types::transaction::{SignedTransaction, TransactionPayload};
use aptos_rust_sdk_types::api_types::{
    module_id::ModuleId,
    transaction::{EntryFunction, RawTransaction},
};
use aptos_rust_sdk_types::state::State;
use privy_rs::PrivyClient;
use serde_json::Value;
use sha3::{Digest, Sha3_256};
use uuid::Uuid;

use crate::{CONFIG, common::zresult::ZResult};
use aptos_rust_sdk_types::api_types::address::AccountAddress;
use bcs;
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct PurchaseData {
    pub buyer_wallet: AccountAddress,
    pub buyer_wallet_id: String,
    pub buyer_wallet_pk: String,
    pub paper_id: Uuid,
}

use super::TransactionSimulation;

/// Submit a purchase transaction to the blockchain using the publish3 Move contract
pub async fn submit_purchase_to_blockchain(
    aptos: &AptosFullnodeClient,
    privy: &PrivyClient,
    data: PurchaseData,
) -> ZResult<(Value, State)> {
    let sequence_number =
        super::find_account_sequence_number(aptos, data.buyer_wallet.to_string()).await;

    let chain_id = 250;
    let module_id = ModuleId::new(
        AccountAddress::from_str(CONFIG.contract_address.as_str()).unwrap(),
        "publication_registry".to_string(),
    );

    let paper_uid_hash: Vec<u8> = Sha3_256::digest(data.paper_id.as_bytes()).to_vec();

    let purchase_entry_function = EntryFunction::new(
        module_id,
        "purchase".to_string(),
        vec![],
        vec![bcs::to_bytes(&paper_uid_hash)?],
    );

    let expiration_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 600;

    let purchase_raw_txn = RawTransaction::new(
        data.buyer_wallet,
        sequence_number,
        TransactionPayload::EntryFunction(purchase_entry_function),
        100_000,
        100,
        expiration_timestamp,
        ChainId::Other(chain_id),
    );

    let sign_response =
        super::sign_with_privy(privy, &data.buyer_wallet_id, &purchase_raw_txn).await?;

    let purchase_authenticator = super::build_authenticator(&data.buyer_wallet_pk, sign_response)?;
    let purchase_signed_txn = SignedTransaction::new(purchase_raw_txn, purchase_authenticator);
    Ok(aptos
        .submit_transaction(purchase_signed_txn)
        .await?
        .into_parts())
}

/// Simulate a purchase transaction to estimate gas costs
pub async fn simulate_purchase_to_blockchain(
    aptos: &AptosFullnodeClient,
    _privy: &PrivyClient,
    data: PurchaseData,
) -> ZResult<TransactionSimulation> {
    let sequence_number =
        super::find_account_sequence_number(aptos, data.buyer_wallet.to_string()).await;

    let chain_id = 250;
    let module_id = ModuleId::new(
        AccountAddress::from_str(CONFIG.contract_address.as_str()).unwrap(),
        "publication_registry".to_string(),
    );

    let paper_uid_hash: Vec<u8> = Sha3_256::digest(data.paper_id.as_bytes()).to_vec();

    let purchase_entry_function = EntryFunction::new(
        module_id,
        "purchase".to_string(),
        vec![],
        vec![bcs::to_bytes(&paper_uid_hash)?],
    );

    let expiration_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 600;

    let purchase_raw_txn = RawTransaction::new(
        data.buyer_wallet,
        sequence_number,
        TransactionPayload::EntryFunction(purchase_entry_function),
        100_000,
        100,
        expiration_timestamp,
        ChainId::Other(chain_id),
    );

    let purchase_authenticator = super::build_simulation_authenticator(&data.buyer_wallet_pk)?;
    let purchase_signed_txn = SignedTransaction::new(purchase_raw_txn, purchase_authenticator);

    let simulation_result = aptos.simulate_transaction(purchase_signed_txn).await?;
    let simulation_value = simulation_result.inner();
    tracing::debug!("Purchase simulation result: {:?}", simulation_value);

    // Helper to parse a simulation value (array of one transaction simulation)
    let parse_simulation = |value: &Value, function_name: &str| -> ZResult<TransactionSimulation> {
        let arr = value
            .as_array()
            .ok_or_else(|| crate::zerror!("Simulation result is not an array"))?;
        let first = arr
            .first()
            .ok_or_else(|| crate::zerror!("Simulation result array is empty"))?;
        let gas_used = first
            .get("gas_used")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .parse::<u64>()
            .map_err(|e| crate::zerror!("Failed to parse gas_used: {}", e))?;
        let gas_unit_price = first
            .get("gas_unit_price")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .parse::<u64>()
            .map_err(|e| crate::zerror!("Failed to parse gas_unit_price: {}", e))?;
        let total_cost_octas = gas_used * gas_unit_price;
        Ok(TransactionSimulation {
            gas_used,
            gas_unit_price,
            total_cost_octas,
            function: function_name.to_string(),
        })
    };

    parse_simulation(simulation_value, "purchase")
}
