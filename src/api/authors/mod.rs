use actix_web::{
    HttpResponse, delete,
    error::{ErrorBadRequest, ErrorConflict, ErrorInternalServerError, ErrorNotFound},
    get, post, put, web,
};
use serde::Deserialize;

use crate::{
    AppState,
    db::sql::{AuthorOperations, PrivyId, models::NewAuthor},
};

use aptos_sdk::types::account_address::AccountAddress;

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/authors")
        .service(create_author)
        .service(list_authors)
        .service(get_author)
        .service(update_author)
        .service(delete_author)
        .service(search_authors);
    conf.service(scope);
}

#[derive(Deserialize)]
pub struct CreateAuthorRequest {
    privy_id: PrivyId,
    name: String,
    email: Option<String>,
    affiliation: Option<String>,
    wallet_address: String,
}

#[post("/create")]
async fn create_author(
    request: web::Json<CreateAuthorRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Err(err) = AccountAddress::from_hex_literal(&request.wallet_address) {
        return Err(ErrorBadRequest(err));
    }

    // Check if wallet address already exists
    let wallet_address_exists = data
        .sql_client
        .author_wallet_address_exists(&request.wallet_address)
        .await
        .map_err(|_| ErrorInternalServerError("Internal server error"))?;

    if wallet_address_exists {
        return Err(ErrorConflict(
            "Author with that wallet address already exists",
        ));
    }

    if let Some(email) = &request.email {
        let email_exists = data
            .sql_client
            .author_email_exists(email)
            .await
            .map_err(|_| ErrorInternalServerError("Internal server error"))?;

        if email_exists {
            return Err(ErrorConflict("Author with that email already exists"));
        }
    }

    let author_by_privy_id = data.sql_client.get_author(&request.privy_id).await;
    if author_by_privy_id.is_ok() {
        return Err(ErrorConflict("Author with that privy_id already exists"));
    }

    let new_author = NewAuthor {
        privy_id: request.privy_id.clone(),
        name: request.name.clone(),
        email: request.email.clone(),
        affiliation: request.affiliation.clone(),
        wallet_address: request.wallet_address.clone(),
    };

    let author = data
        .sql_client
        .create_author(&new_author)
        .await
        .map_err(|err| {
            tracing::error!("Error creating author: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(author))
}

#[get("/{privy_id}")]
async fn get_author(
    privy_id: web::Path<PrivyId>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let author = data.sql_client.get_author(&privy_id).await.map_err(|err| {
        tracing::error!("Error retrieving author: {}", err);
        match err {
            sqlx::Error::RowNotFound => ErrorNotFound("Author not found"),
            _ => ErrorInternalServerError("Internal server error"),
        }
    })?;

    Ok(HttpResponse::Ok().json(author))
}

#[derive(Deserialize)]
pub struct UpdateAuthorRequest {
    name: Option<String>,
    email: Option<String>,
    affiliation: Option<String>,
    wallet_address: Option<String>,
}

#[put("/{privy_id}")]
async fn update_author(
    privy_id: web::Path<PrivyId>,
    request: web::Json<UpdateAuthorRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Some(wallet_address) = &request.wallet_address {
        if let Err(err) = AccountAddress::from_hex_literal(wallet_address) {
            return Err(ErrorBadRequest(err));
        }

        let wallet_address_exists = data
            .sql_client
            .author_wallet_address_exists(wallet_address)
            .await
            .map_err(|_| ErrorInternalServerError("Internal server error"))?;

        if wallet_address_exists {
            let existing_author = data
                .sql_client
                .get_author_by_wallet_address(wallet_address)
                .await;
            match existing_author {
                Ok(existing) => {
                    if existing.privy_id != *privy_id {
                        return Err(ErrorConflict(
                            "Another author with that wallet address already exists",
                        ));
                    }
                }
                Err(sqlx::Error::RowNotFound) => {
                    // Wallet address doesn't exist, that's fine
                }
                Err(err) => {
                    tracing::error!("Error checking author wallet address: {}", err);
                    return Err(ErrorInternalServerError("Internal server error"));
                }
            }
        }
    }

    // Check if new email already exists (if email is being updated)
    if let Some(email) = &request.email {
        let email_exists = data
            .sql_client
            .author_email_exists(email)
            .await
            .map_err(|_| ErrorInternalServerError("Internal server error"))?;

        if email_exists {
            // Check if it's the same author
            let existing_author = data.sql_client.get_author_by_email(email).await;
            match existing_author {
                Ok(existing) => {
                    if existing.privy_id != *privy_id {
                        return Err(ErrorConflict(
                            "Another author with that email already exists",
                        ));
                    }
                }
                Err(sqlx::Error::RowNotFound) => {
                    // Email doesn't exist, that's fine
                }
                Err(err) => {
                    tracing::error!("Error checking author email: {}", err);
                    return Err(ErrorInternalServerError("Internal server error"));
                }
            }
        }
    }

    let result = data
        .sql_client
        .update_author(
            &privy_id,
            request.name.as_deref(),
            request.email.as_deref(),
            request.affiliation.as_deref(),
            request.wallet_address.as_deref(),
        )
        .await
        .map_err(|err| {
            tracing::error!("Error updating author: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if result.rows_affected() == 0 {
        return Err(ErrorNotFound("Author not found"));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Author updated successfully"
    })))
}

#[delete("/{privy_id}")]
async fn delete_author(
    privy_id: web::Path<PrivyId>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let result = data
        .sql_client
        .delete_author(&privy_id)
        .await
        .map_err(|err| {
            tracing::error!("Error deleting author: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if result.rows_affected() == 0 {
        return Err(ErrorNotFound("Author not found"));
    }

    Ok(HttpResponse::NoContent().finish())
}

#[get("/list")]
async fn list_authors(
    data: web::Data<AppState>,
    query: web::Query<ListAuthorsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let authors = data
        .sql_client
        .list_authors(query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error listing authors: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    let total_count = data.sql_client.count_authors().await.map_err(|err| {
        tracing::error!("Error counting authors: {}", err);
        ErrorInternalServerError("Internal server error")
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "authors": authors,
        "total": total_count,
        "page": query.page.unwrap_or(1),
        "limit": query.limit.unwrap_or(20)
    })))
}

#[derive(Deserialize)]
struct ListAuthorsQuery {
    page: Option<i64>,
    limit: Option<i64>,
}

#[get("/search")]
async fn search_authors(
    data: web::Data<AppState>,
    query: web::Query<SearchAuthorsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let authors = data
        .sql_client
        .search_authors_by_name(&query.name, query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error searching authors: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(authors))
}

#[derive(Deserialize)]
struct SearchAuthorsQuery {
    name: String,
    page: Option<i64>,
    limit: Option<i64>,
}
