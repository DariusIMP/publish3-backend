use actix_web::{
    web,
    HttpResponse,
    get, post,
    error::ErrorInternalServerError,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    blockchain::purchases::{PurchaseData, simulate_purchase_to_blockchain},
    db::sql::{PurchaseOperations, UserOperations, PublicationOperations, WalletOperations},
};
use aptos_rust_sdk_types::api_types::address::AccountAddress;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct ListPurchasesQuery {
    page: Option<i64>,
    limit: Option<i64>,
}

#[get("/user/{user_id}")]
pub async fn list_user_purchases(
    user_id: web::Path<uuid::Uuid>,
    data: web::Data<AppState>,
    query: web::Query<ListPurchasesQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let purchases = data
        .sql_client
        .list_purchases_by_user(&user_id, query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error listing purchases for user {}: {}", user_id, err);
            ErrorInternalServerError("Internal server error")
        })?;

    let total_count = data
        .sql_client
        .count_purchases_by_user(&user_id)
        .await
        .map_err(|err| {
            tracing::error!("Error counting purchases for user {}: {}", user_id, err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "purchases": purchases,
        "total": total_count,
        "page": query.page.unwrap_or(1),
        "limit": query.limit.unwrap_or(20),
    })))
}

#[get("/count")]
pub async fn count_purchases(
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let total_count = data.sql_client.count_purchases().await.map_err(|err| {
        tracing::error!("Error counting purchases: {}", err);
        ErrorInternalServerError("Internal server error")
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "total": total_count,
    })))
}

#[derive(Debug, Deserialize)]
pub struct SimulatePurchaseRequest {
    publication_id: Uuid,
}

#[post("/simulate")]
pub async fn simulate_purchase(
    req: actix_web::HttpRequest,
    data: web::Data<AppState>,
    request: web::Json<SimulatePurchaseRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let claims = crate::auth::privy::get_privy_claims(&req).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("Valid Privy authentication token required")
    })?;

    let privy_id = claims.sub;
    let user = data
        .sql_client
        .get_user_by_privy_id(privy_id.clone())
        .await
        .map_err(|err| {
            tracing::error!("Error finding user for privy_id {}: {}", privy_id, err);
            actix_web::error::ErrorInternalServerError("User not found")
        })?;

    // Get publication details
    let publication = data
        .sql_client
        .get_publication(request.publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving publication: {}", err);
            match err {
                sqlx::Error::RowNotFound => actix_web::error::ErrorNotFound("Publication not found"),
                _ => actix_web::error::ErrorInternalServerError("Internal server error"),
            }
        })?;

    let buyer_wallet = data
        .sql_client
        .get_primary_wallet(&user.id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving user wallet: {}", err);
            actix_web::error::ErrorInternalServerError("Internal server error")
        })?;

    let buyer_wallet_pk = data
        .privy_client
        .wallets()
        .get(&buyer_wallet.wallet_id)
        .await
        .map(|wallet| wallet.public_key.clone())
        .map_err(|err| {
            tracing::error!("Failed to get wallet from Privy: {}", err);
            actix_web::error::ErrorInternalServerError("Internal server error")
        })?
        .ok_or_else(|| {
            tracing::error!("Wallet lacks public key");
            actix_web::error::ErrorInternalServerError("Wallet lacks public key")
        })?;

    let buyer_wallet_address =
        AccountAddress::from_str(&buyer_wallet.wallet_address).map_err(|err| {
            tracing::error!("Error parsing wallet address: {}", err);
            actix_web::error::ErrorInternalServerError("Invalid wallet address")
        })?;

    let purchase_data = PurchaseData {
        buyer_wallet: buyer_wallet_address,
        buyer_wallet_id: buyer_wallet.wallet_id.clone(),
        buyer_wallet_pk,
        paper_id: publication.id,
    };

    // Simulate purchase transaction
    let simulation = simulate_purchase_to_blockchain(
        &data.aptos_client,
        &data.privy_client,
        purchase_data,
    )
    .await
    .map_err(|err| {
        tracing::error!("Failed to simulate purchase to blockchain: {}", err);
        actix_web::error::ErrorInternalServerError("Blockchain simulation failed")
    })?;

    Ok(HttpResponse::Ok().json(simulation))
}
