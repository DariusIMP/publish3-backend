use crate::{
    AppState,
    blockchain::{
        PublicationData,
        purchases::{PurchaseData, submit_purchase_to_blockchain},
        simulate_publication_to_blockchain, submit_publication_to_blockchain,
    },
    common::zresult::ZResult,
    db::{
        s3::S3Key,
        sql::{
            CitationOperations, PublicationAuthorOperations, PublicationOperations,
            PurchaseOperations, SqlClient, UserOperations, WalletOperations,
            models::NewPublication,
        },
    },
    zerror,
};
use actix_multipart::form::{MultipartForm, tempfile::TempFile, text::Text};
use actix_web::{
    HttpResponse, delete,
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound},
    get, post, put, web,
};

use aptos_rust_sdk_types::api_types::address::{AccountAddress, AccountAddressParseError};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{
    io::{BufReader, Read},
    str::FromStr,
};
use uuid::Uuid;

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/publications")
        .service(create_publication)
        .service(simulate_publication)
        .service(list_publications)
        .service(count_publications)
        .service(list_publications_by_user)
        .service(search_publications_by_title)
        .service(search_publications_by_tag)
        .service(get_publication)
        .service(update_publication)
        .service(delete_publication)
        .service(get_publication_authors_handler)
        .service(get_publication_citations)
        .service(get_cited_by)
        .service(get_publication_pdf_url)
        .service(check_publication_access_endpoint)
        .service(update_publication_transaction_status)
        .service(purchase_publication);
    conf.service(scope);
}

#[cfg(test)]
mod tests;

#[derive(MultipartForm)]
#[allow(non_snake_case)]
pub struct CreatePublicationForm {
    title: Text<String>,
    about: Text<String>,
    tags: Text<String>,
    authors: Text<String>,
    citations: Option<Text<String>>,
    price: Text<i64>,
    file: TempFile,
}

#[post("/create")]
async fn create_publication(
    req: actix_web::HttpRequest,
    MultipartForm(form): MultipartForm<CreatePublicationForm>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let claims = crate::auth::privy::get_privy_claims(&req).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("Valid Privy authentication token required")
    })?;
    let user_id = claims.sub;

    let user = data
        .sql_client
        .get_user_by_privy_id(user_id.clone())
        .await
        .map_err(|err| {
            tracing::error!("Error finding user for privy_id {}: {}", user_id, err);
            ErrorInternalServerError("User not found")
        })?;

    let response = handle_publication(&user.id, &form, &data)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(response))
}

#[post("/simulate")]
async fn simulate_publication(
    req: actix_web::HttpRequest,
    MultipartForm(form): MultipartForm<CreatePublicationForm>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let claims = crate::auth::privy::get_privy_claims(&req).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("Valid Privy authentication token required")
    })?;
    let user_id = claims.sub;

    preview_publication(&user_id, &form, &data)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(()))
}

#[get("/{publication_id}")]
async fn get_publication(
    req: actix_web::HttpRequest,
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    crate::auth::privy::get_privy_claims(&req).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("Valid Privy authentication token required")
    })?;

    let publication = data
        .sql_client
        .get_publication(*publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving publication: {}", err);
            match err {
                sqlx::Error::RowNotFound => ErrorNotFound("Publication not found"),
                _ => ErrorInternalServerError("Internal server error"),
            }
        })?;

    let authors_with_details = data
        .sql_client
        .get_publication_authors_with_details(*publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving publication authors: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    let authors: Vec<serde_json::Value> = authors_with_details
        .into_iter()
        .map(|author_detail| {
            serde_json::json!({
                "privy_id": author_detail.author_id,
                "name": author_detail.author_name,
                "email": author_detail.author_email,
                "affiliation": author_detail.author_affiliation,
            })
        })
        .collect();

    let citation_count = data
        .sql_client
        .get_citation_count(*publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving citation count: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    let response = serde_json::json!({
        "id": publication.id,
        "user_id": publication.user_id,
        "title": publication.title,
        "about": publication.about,
        "tags": publication.tags,
        "created_at": publication.created_at,
        "updated_at": publication.updated_at,
        "authors": authors,
        "citation_count": citation_count,
    });

    Ok(HttpResponse::Ok().json(response))
}

#[derive(MultipartForm)]
#[allow(non_snake_case)]
pub struct UpdatePublicationForm {
    userId: Option<Text<Uuid>>,
    title: Option<Text<String>>,
    about: Option<Text<String>>,
    tags: Option<Text<String>>, // JSON array string like ["tag1", "tag2"]
    file: Option<TempFile>,
}

#[put("/{publication_id}")]
async fn update_publication(
    publication_id: web::Path<Uuid>,
    MultipartForm(form): MultipartForm<UpdatePublicationForm>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Parse tags from JSON array string if provided
    let tags = if let Some(tags_text) = &form.tags {
        match serde_json::from_str::<Vec<String>>(&tags_text.0) {
            Ok(tags) => Some(tags),
            Err(err) => {
                tracing::error!("Failed to parse tags JSON: {}", err);
                return Err(ErrorBadRequest("Invalid tags format. Expected JSON array"));
            }
        }
    } else {
        None
    };

    // Handle file upload if present
    let mut s3key = None;
    if let Some(file) = form.file {
        // Upload file to S3 using the storage bucket
        let file_name = file
            .file_name
            .as_ref()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "unnamed.pdf".to_string());

        let publication_uuid = Uuid::new_v4();
        let s3_directory = format!("publications/{}", publication_uuid);
        let s3_path = format!("{}/{}", s3_directory, file_name);

        data.s3_client
            .store_file(&file, Some(s3_directory.into()))
            .await
            .map_err(|err| {
                tracing::error!("Error uploading file to S3: {}", err);
                ErrorInternalServerError("Failed to upload file")
            })?;

        s3key = Some(s3_path);
    }

    let result = data
        .sql_client
        .update_publication(
            *publication_id,
            form.userId.as_ref().map(|u| &u.0),
            form.title.as_ref().map(|t| t.0.as_str()),
            form.about.as_ref().map(|a| a.0.as_str()),
            tags.as_deref(),
            s3key.as_deref(),
        )
        .await
        .map_err(|err| {
            tracing::error!("Error updating publication: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if result.rows_affected() == 0 {
        return Err(ErrorNotFound("Publication not found"));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Publication updated successfully"
    })))
}

#[delete("/{publication_id}")]
async fn delete_publication(
    req: actix_web::HttpRequest,
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Get user ID from authenticated user
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
            ErrorInternalServerError("User not found")
        })?;

    delete_publication_internal(&data, *publication_id, &user.id).await
}

/// Internal function to delete a publication (used for rollback)
async fn delete_publication_internal(
    data: &AppState,
    publication_id: Uuid,
    user_id: &Uuid,
) -> Result<HttpResponse, actix_web::Error> {
    // First get the publication to check if it has an S3 file
    let publication = data
        .sql_client
        .get_publication(publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving publication: {}", err);
            match err {
                sqlx::Error::RowNotFound => ErrorNotFound("Publication not found"),
                _ => ErrorInternalServerError("Internal server error"),
            }
        })?;

    // Check if the user is authorized to delete this publication
    if publication.user_id != *user_id {
        return Err(actix_web::error::ErrorForbidden(
            "You are not authorized to delete this publication",
        ));
    }

    // Check if the publication is in PENDING_ONCHAIN status
    if publication.status != "PENDING_ONCHAIN" {
        return Err(ErrorBadRequest(
            "Publication can only be deleted during rollback when status is PENDING_ONCHAIN",
        ));
    }

    if !publication.s3key.is_empty()
        && let Err(err) = data.s3_client.delete_file(S3Key(publication.s3key)).await
    {
        tracing::warn!("Failed to delete S3 file: {}", err);
        // Continue with database deletion even if S3 deletion fails
    }

    let result = data
        .sql_client
        .delete_publication(publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error deleting publication: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if result.rows_affected() == 0 {
        return Err(ErrorNotFound("Publication not found"));
    }

    Ok(HttpResponse::NoContent().finish())
}

#[get("/list")]
async fn list_publications(
    data: web::Data<AppState>,
    query: web::Query<ListPublicationsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let publications_with_authors = data
        .sql_client
        .list_publications_with_authors(query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error listing publications: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    let total_count = data.sql_client.count_publications().await.map_err(|err| {
        tracing::error!("Error counting publications: {}", err);
        ErrorInternalServerError("Internal server error")
    })?;

    // Transform to include authors and citation counts in each publication
    let mut publications: Vec<serde_json::Value> = Vec::new();

    for (publication, authors) in publications_with_authors {
        // Get citation count for this publication
        let citation_count = data
            .sql_client
            .get_citation_count(publication.id)
            .await
            .map_err(|err| {
                tracing::error!(
                    "Error retrieving citation count for publication {}: {}",
                    publication.id,
                    err
                );
                ErrorInternalServerError("Internal server error")
            })?;

        publications.push(serde_json::json!({
            "id": publication.id,
            "user_id": publication.user_id,
            "title": publication.title,
            "about": publication.about,
            "tags": publication.tags,
            "created_at": publication.created_at,
            "updated_at": publication.updated_at,
            "authors": authors,
            "citation_count": citation_count,
        }));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "publications": publications,
        "total": total_count,
        "page": query.page.unwrap_or(1),
        "limit": query.limit.unwrap_or(20)
    })))
}

#[get("/count")]
async fn count_publications(data: web::Data<AppState>) -> Result<HttpResponse, actix_web::Error> {
    let total_count = data.sql_client.count_publications().await.map_err(|err| {
        tracing::error!("Error counting publications: {}", err);
        ErrorInternalServerError("Internal server error")
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "total": total_count,
    })))
}

#[derive(Deserialize)]
struct ListPublicationsQuery {
    page: Option<i64>,
    limit: Option<i64>,
}

#[get("/user/{id}")]
async fn list_publications_by_user(
    id: web::Path<uuid::Uuid>,
    data: web::Data<AppState>,
    query: web::Query<ListPublicationsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let publications = data
        .sql_client
        .list_publications_by_user(&id, query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error listing user publications: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    let total_count = data
        .sql_client
        .count_publications_by_user(&id)
        .await
        .map_err(|err| {
            tracing::error!("Error counting user publications: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "publications": publications,
        "total": total_count,
        "page": query.page.unwrap_or(1),
        "limit": query.limit.unwrap_or(20)
    })))
}

#[get("/search/title")]
async fn search_publications_by_title(
    data: web::Data<AppState>,
    query: web::Query<SearchPublicationsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    if query.query.is_empty() {
        return Err(ErrorBadRequest("Search query cannot be empty"));
    }

    let publications = data
        .sql_client
        .search_publications_by_title(&query.query, query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error searching publications by title: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(publications))
}

#[get("/search/tag")]
async fn search_publications_by_tag(
    data: web::Data<AppState>,
    query: web::Query<SearchByTagQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    if query.tag.is_empty() {
        return Err(ErrorBadRequest("Tag cannot be empty"));
    }

    let publications = data
        .sql_client
        .search_publications_by_tag(&query.tag, query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error searching publications by tag: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(publications))
}

#[derive(Deserialize)]
struct SearchPublicationsQuery {
    query: String,
    page: Option<i64>,
    limit: Option<i64>,
}

#[derive(Deserialize)]
struct SearchByTagQuery {
    tag: String,
    page: Option<i64>,
    limit: Option<i64>,
}

#[get("/{publication_id}/authors")]
async fn get_publication_authors_handler(
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let authors =
        PublicationAuthorOperations::get_publication_authors(&*data.sql_client, *publication_id)
            .await
            .map_err(|err| {
                tracing::error!("Error retrieving publication authors: {}", err);
                match err {
                    sqlx::Error::RowNotFound => ErrorNotFound("Publication not found"),
                    _ => ErrorInternalServerError("Internal server error"),
                }
            })?;

    Ok(HttpResponse::Ok().json(authors))
}

#[get("/{publication_id}/citations")]
async fn get_publication_citations(
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let citations = data
        .sql_client
        .get_publication_citations(*publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving publication citations: {}", err);
            match err {
                sqlx::Error::RowNotFound => ErrorNotFound("Publication not found"),
                _ => ErrorInternalServerError("Internal server error"),
            }
        })?;

    Ok(HttpResponse::Ok().json(citations))
}

#[get("/{publication_id}/cited-by")]
async fn get_cited_by(
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let cited_by = data
        .sql_client
        .get_cited_by(*publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving publications that cite this one: {}", err);
            match err {
                sqlx::Error::RowNotFound => ErrorNotFound("Publication not found"),
                _ => ErrorInternalServerError("Internal server error"),
            }
        })?;

    Ok(HttpResponse::Ok().json(cited_by))
}

#[get("/{publication_id}/pdf-url")]
async fn get_publication_pdf_url(
    req: actix_web::HttpRequest,
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
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
            ErrorInternalServerError("User not found")
        })?;

    check_publication_access(&data, &user.id, *publication_id).await?;

    let publication = data
        .sql_client
        .get_publication(*publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving publication: {}", err);
            match err {
                sqlx::Error::RowNotFound => ErrorNotFound("Publication not found"),
                _ => ErrorInternalServerError("Internal server error"),
            }
        })?;

    if publication.s3key.is_empty() {
        return Err(ErrorNotFound(
            "Publication does not have an associated PDF file",
        ));
    }

    let pdf_url = data
        .s3_client
        .get_file_url(&publication.s3key, &crate::db::s3::S3Bucket::Storage)
        .await
        .map_err(|err| {
            tracing::error!("Error generating PDF URL: {}", err);
            ErrorInternalServerError("Failed to generate PDF URL")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "pdf_url": pdf_url,
        "expires_in": "5 minutes" // Presigned URL expires in 5 minutes
    })))
}

#[get("/{publication_id}/access")]
async fn check_publication_access_endpoint(
    req: actix_web::HttpRequest,
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
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
            ErrorInternalServerError("User not found")
        })?;

    let has_access = check_publication_access(&data, &user.id, *publication_id)
        .await
        .map_or_else(|_| false, |_| true);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "has_access": has_access,
        "publication_id": *publication_id,
    })))
}

/// Helper function to check if a user has access to a publication
/// Returns Ok(()) if user has access, Err(actix_web::Error) if not
async fn check_publication_access(
    data: &AppState,
    user_id: &Uuid,
    publication_id: Uuid,
) -> Result<(), actix_web::Error> {
    let publication = data
        .sql_client
        .get_publication(publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving publication: {}", err);
            match err {
                sqlx::Error::RowNotFound => ErrorNotFound("Publication not found"),
                _ => ErrorInternalServerError("Internal server error"),
            }
        })?;

    // Check if user is the author
    let is_author = publication.user_id == *user_id;

    // Get authors with details to check if user is a co-author
    let authors_with_details = data
        .sql_client
        .get_publication_authors_with_details(publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving publication authors: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    // Check if user is one of the authors (from publication_authors table)
    let is_coauthor = authors_with_details
        .iter()
        .any(|author_detail| author_detail.author_id == *user_id);

    // Check if user has purchased this publication
    let has_purchased = data
        .sql_client
        .has_user_purchased_publication(user_id, publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error checking purchase status: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    // User must be either author/coauthor or have purchased the publication
    if !is_author && !is_coauthor && !has_purchased {
        return Err(actix_web::error::ErrorForbidden(
            "You do not have access to this publication. Please purchase it first.",
        ));
    }

    Ok(())
}

/// Helper function to process citations for a publication
async fn process_citations(sql_client: &SqlClient, publication_id: Uuid, citations: &Text<String>) {
    let citations = serde_json::from_str::<Vec<Uuid>>(&citations.0).unwrap_or_default();

    for cited_publication_id in citations {
        if cited_publication_id == publication_id {
            tracing::warn!(
                "Publication {} attempted to cite itself, skipping",
                publication_id
            );
            continue;
        }

        let existing_citation = sql_client
            .get_citation_by_publications(publication_id, cited_publication_id)
            .await;

        match existing_citation {
            Ok(Some(_)) => {
                tracing::warn!(
                    "Citation already exists between {} and {}, skipping",
                    publication_id,
                    cited_publication_id
                );
                continue;
            }
            Ok(None) => {
                let new_citation = crate::db::sql::models::NewCitation {
                    citing_publication_id: publication_id,
                    cited_publication_id,
                };

                if let Err(err) = sql_client.create_citation(&new_citation).await {
                    tracing::error!(
                        "Error creating citation from {} to {}: {}",
                        publication_id,
                        cited_publication_id,
                        err
                    );
                }
            }
            Err(err) => {
                tracing::error!("Error checking existing citation: {}", err);
            }
        }
    }
}

#[derive(Deserialize)]
struct UpdateTransactionStatusRequest {
    status: String,
    transaction_hash: Option<String>,
}

#[put("/{publication_id}/transaction-status")]
async fn update_publication_transaction_status(
    req: actix_web::HttpRequest,
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
    request: web::Json<UpdateTransactionStatusRequest>,
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
            ErrorInternalServerError("User not found")
        })?;

    let publication = data
        .sql_client
        .get_publication(*publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving publication: {}", err);
            match err {
                sqlx::Error::RowNotFound => ErrorNotFound("Publication not found"),
                _ => ErrorInternalServerError("Internal server error"),
            }
        })?;

    if publication.user_id != user.id {
        return Err(actix_web::error::ErrorForbidden(
            "You are not authorized to update this publication",
        ));
    }

    let valid_statuses = ["PENDING_ONCHAIN", "PUBLISHED", "FAILED"];
    if !valid_statuses.contains(&request.status.as_str()) {
        return Err(ErrorBadRequest(format!(
            "Invalid status. Must be one of: {}",
            valid_statuses.join(", ")
        )));
    }

    let result = data
        .sql_client
        .update_publication_transaction_status(
            *publication_id,
            &request.status,
            request.transaction_hash.as_deref(),
        )
        .await
        .map_err(|err| {
            tracing::error!("Error updating publication transaction status: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if result.rows_affected() == 0 {
        return Err(ErrorNotFound("Publication not found"));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Publication transaction status updated successfully"
    })))
}

#[post("/{publication_id}/purchase")]
async fn purchase_publication(
    req: actix_web::HttpRequest,
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
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
            ErrorInternalServerError("User not found")
        })?;

    // Get publication details
    let publication = data
        .sql_client
        .get_publication(*publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving publication: {}", err);
            match err {
                sqlx::Error::RowNotFound => ErrorNotFound("Publication not found"),
                _ => ErrorInternalServerError("Internal server error"),
            }
        })?;

    // Check if user already purchased this publication
    let already_purchased = data
        .sql_client
        .has_user_purchased_publication(&user.id, *publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error checking purchase status: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if already_purchased {
        return Err(ErrorBadRequest(
            "You have already purchased this publication",
        ));
    }

    // Get buyer's wallet
    let buyer_wallet = data
        .sql_client
        .get_primary_wallet(&user.id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving user wallet: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    // Get buyer's wallet public key from Privy
    let buyer_wallet_pk = data
        .privy_client
        .wallets()
        .get(&buyer_wallet.wallet_id)
        .await
        .map(|wallet| wallet.public_key.clone())
        .map_err(|err| {
            tracing::error!("Failed to get wallet from Privy: {}", err);
            ErrorInternalServerError("Internal server error")
        })?
        .ok_or_else(|| {
            tracing::error!("Wallet lacks public key");
            ErrorInternalServerError("Wallet lacks public key")
        })?;

    let buyer_wallet_address =
        AccountAddress::from_str(&buyer_wallet.wallet_address).map_err(|err| {
            tracing::error!("Error parsing wallet address: {}", err);
            ErrorInternalServerError("Invalid wallet address")
        })?;

    let purchase_data = PurchaseData {
        buyer_wallet: buyer_wallet_address,
        buyer_wallet_id: buyer_wallet.wallet_id.clone(),
        buyer_wallet_pk,
        paper_id: publication.id,
    };

    // Submit purchase to blockchain
    let (value, _state) =
        submit_purchase_to_blockchain(&data.aptos_client, &data.privy_client, purchase_data)
            .await
            .map_err(|err| {
                tracing::error!("Failed to submit purchase to blockchain: {}", err);
                ErrorInternalServerError("Blockchain transaction failed")
            })?;

    let transaction_hash = value
        .get("hash")
        .and_then(|h| h.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            tracing::error!("Transaction response missing hash");
            ErrorInternalServerError("Transaction response missing hash")
        })?;

    // Record purchase in database
    let new_purchase = crate::db::sql::models::NewPurchase {
        user_id: user.id,
        publication_id: *publication_id,
        status: Some("PAID".to_string()),
        transaction_hash: Some(transaction_hash.clone()),
    };

    data.sql_client
        .create_purchase(&new_purchase)
        .await
        .map_err(|err| {
            tracing::error!("Failed to record purchase in database: {}", err);
            ErrorInternalServerError("Failed to record purchase")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Purchase completed successfully",
        "transaction_hash": transaction_hash,
        "publication_id": *publication_id,
    })))
}

#[derive(Serialize)]
struct PublicationResponse {
    id: Uuid,
    transaction_hash: String,
}

async fn preview_publication(
    user_id: &String,
    form: &CreatePublicationForm,
    data: &AppState,
) -> ZResult<()> {
    // Get user by privy_id to get UUID
    let user = data
        .sql_client
        .get_user_by_privy_id(user_id.clone())
        .await
        .map_err(|err| zerror!("Failed to find user for privy_id {}: {}", user_id, err))?;

    let authors =
        parse_authors(&form.authors).map_err(|err| zerror!("Failed to parse authors: {}", err))?;

    let id = Uuid::new_v4();
    let publication_data = prepare_blockchain_transaction(data, &user.id, id, &authors, form)
        .await
        .map_err(|err| zerror!(err))?;

    let _ = simulate_publication_to_blockchain(
        &data.aptos_client,
        &data.privy_client,
        &publication_data,
    )
    .await
    .map_err(|err| {
        let error_msg = format!("Failed to submit publication to blockchain: {}", err);
        tracing::debug!(error_msg);
        zerror!(error_msg)
    });

    Ok(())
}

async fn handle_publication(
    user_id: &Uuid,
    form: &CreatePublicationForm,
    data: &AppState,
) -> ZResult<PublicationResponse> {
    let authors =
        parse_authors(&form.authors).map_err(|err| zerror!("Failed to parse authors: {}", err))?;

    let publication = store_publication(form, user_id, &authors, data)
        .await
        .map_err(|err| zerror!("Failed to store publication: {}", err))?;

    let publication_data =
        prepare_blockchain_transaction(data, user_id, publication.id, &authors, form)
            .await
            .map_err(|err| zerror!(err))?;

    let response =
        submit_publication_to_blockchain(&data.aptos_client, &data.privy_client, publication_data)
            .await
            .map_err(|err| {
                let error_msg = format!("Failed to submit publication to blockchain: {}", err);
                tracing::debug!(error_msg);
                zerror!(error_msg)
            });

    match response {
        Ok((value, _state)) => {
            tracing::debug!("Publication to blockchain successful: {:?}", value);

            let transaction_hash = value
                .get("hash")
                .and_then(|h| h.as_str())
                .map(|s| s.to_string());

            let result = data
                .sql_client
                .update_publication_transaction_status(
                    publication.id,
                    "PUBLISHED",
                    transaction_hash.as_deref(),
                )
                .await;

            if let Err(err) = result {
                tracing::error!("Failed to update publication status on DB: {}", err);
            }
            Ok(PublicationResponse {
                id: publication.id,
                transaction_hash: transaction_hash.unwrap(),
            })
        }
        Err(err) => {
            let _ = delete_publication_internal(data, publication.id, user_id).await;
            Err(zerror!("The publication transaction failed: {}", err))
        }
    }
}

async fn prepare_blockchain_transaction(
    data: &AppState,
    user_id: &Uuid,
    publication_id: Uuid,
    authors: &[Uuid],
    form: &CreatePublicationForm,
) -> ZResult<PublicationData> {
    let user_wallet = data
        .sql_client
        .get_primary_wallet(user_id)
        .await
        .map_err(|err| zerror!("Error retrieving user wallet from DB: {}", err))?;

    let user_wallet_account = AccountAddress::from_str(&user_wallet.wallet_address)
        .map_err(|err| zerror!("Error parsing user wallet address: {}", err))?;

    let authors_wallets = data
        .sql_client
        .get_primary_wallets(authors)
        .await
        .map_err(|err| zerror!("Error fetching authors wallets from DB: {}", err))?;

    let authors_wallets_accounts = authors_wallets
        .iter()
        .map(|wallet| AccountAddress::from_str(&wallet.wallet_address))
        .collect::<Result<Vec<AccountAddress>, AccountAddressParseError>>()
        .map_err(|err| zerror!("Error parsing author wallet addresses: {}", err))?;

    let user_wallet_pk = data
        .privy_client
        .wallets()
        .get(&user_wallet.wallet_id)
        .await
        .map(|wallet| wallet.public_key.clone())
        .map_err(|err| zerror!("Failed to get wallet from Privy: {}", err))?
        .ok_or(zerror!("Wallet lacks public key!"))?;

    let paper_hash =
        hash_file_sha3_256(&form.file).map_err(|err| zerror!("Failed to hash file: {}", err))?;

    let paper_uid_hash = Sha3_256::digest(publication_id.as_bytes()).into();

    Ok(PublicationData {
        paper_uid_hash,
        paper_hash,
        user_wallet: user_wallet_account,
        user_wallet_id: user_wallet.wallet_id.clone(),
        user_wallet_pk,
        author_wallets: authors_wallets_accounts,
        price: form.price.0 as u64,
    })
}

fn hash_file_sha3_256(temp_file: &TempFile) -> ZResult<[u8; 32]> {
    let mut reader = BufReader::new(temp_file.file.as_file());
    let mut hasher = Sha3_256::default();

    let mut buffer = [0u8; 8192];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest::DynDigest::update(&mut hasher, &buffer[..n]);
    }

    let result = hasher.finalize();
    Ok(result.into())
}

async fn store_publication(
    form: &CreatePublicationForm,
    user_id: &Uuid,
    authors: &[Uuid],
    data: &AppState,
) -> ZResult<crate::db::sql::models::Publication> {
    let tags = serde_json::from_str::<Vec<String>>(&form.tags.0)?;

    let file_name = form
        .file
        .file_name
        .as_ref()
        .map(|n| n.to_string())
        .unwrap_or_else(|| "unnamed.pdf".to_string());

    let publication_uuid = Uuid::new_v4();
    let s3_directory = format!("publications/{}", publication_uuid);
    let s3key = format!("{}/{}", s3_directory, file_name);

    data.s3_client
        .store_file(&form.file, Some(s3_directory.into()))
        .await
        .map_err(|err| zerror!("Error uploading file to S3: {}", err))?;

    let new_publication = NewPublication {
        user_id: *user_id,
        title: form.title.0.clone(),
        about: form.about.0.clone(),
        tags,
        s3key,
        price: form.price.0,
    };

    // TODO: make these SQL operations atomic
    let publication = data
        .sql_client
        .create_publication(&new_publication)
        .await
        .map_err(|err| zerror!("Failed to create publication entry: {}", err))?;

    data.sql_client
        .set_publication_authors(publication.id, authors)
        .await
        .map_err(|err| zerror!("Failed to set publication authors: {}", err))?;

    if let Some(citations) = &form.citations {
        process_citations(&data.sql_client, publication.id, citations).await;
    }

    Ok(publication)
}

fn parse_authors(authors_text: &Text<String>) -> ZResult<Vec<Uuid>> {
    serde_json::from_str::<Vec<Uuid>>(authors_text).map_err(|err| {
        zerror!(
            "Error parsing authors: {}. Expected JSON array of author IDs.",
            err
        )
    })
}
