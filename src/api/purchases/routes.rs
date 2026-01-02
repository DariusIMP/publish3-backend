use actix_web::{
    web,
    HttpResponse,
    get,
    error::ErrorInternalServerError,
};
use serde::Deserialize;

use crate::{
    AppState,
    db::sql::PurchaseOperations,
};

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
