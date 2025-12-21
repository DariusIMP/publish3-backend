use crate::{
    AppState, CONFIG,
    blockchain::{CapabilitySigner, SignedCapability},
    db::sql::{
        AuthorOperations, CitationOperations, PrivyId, PublicationAuthorOperations,
        PublicationOperations, SqlClient, models::NewPublication,
    },
};
use actix_multipart::form::{MultipartForm, tempfile::TempFile, text::Text};
use actix_web::{
    HttpResponse, delete,
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound},
    get, post, put, web,
};
use serde::Deserialize;
use sha3::{Digest, Sha3_256};
use std::io::{BufReader, Read};
use uuid::Uuid;

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/publications")
        .service(create_publication)
        .service(list_publications)
        .service(list_publications_by_user)
        .service(search_publications_by_title)
        .service(search_publications_by_tag)
        .service(get_publication)
        .service(update_publication)
        .service(delete_publication)
        .service(get_publication_authors_handler)
        .service(get_publication_citations)
        .service(get_cited_by)
        .service(get_publication_pdf_url);
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
    citation_royalty_bps: Text<i64>,
    file: TempFile,
}

#[post("/create")]
async fn create_publication(
    req: actix_web::HttpRequest,
    MultipartForm(form): MultipartForm<CreatePublicationForm>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Get user ID from authenticated user
    let claims = crate::auth::privy::get_privy_claims(&req).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("Valid Privy authentication token required")
    })?;

    let user_id = claims.sub;
    let tags = match serde_json::from_str::<Vec<String>>(&form.tags.0) {
        Ok(tags) => tags,
        Err(err) => {
            tracing::error!("Failed to parse tags JSON: {}", err);
            return Err(ErrorBadRequest("Invalid tags format. Expected JSON array"));
        }
    };

    // Parse authors from JSON array string (now mandatory)
    let authors = match serde_json::from_str::<Vec<PrivyId>>(&form.authors.0) {
        Ok(authors) => authors,
        Err(err) => {
            tracing::error!("Failed to parse authors JSON: {}", err);
            return Err(ErrorBadRequest(
                "Invalid authors format. Expected JSON array of author IDs",
            ));
        }
    };

    let citations = match &form.citations {
        Some(citations_text) => match serde_json::from_str::<Vec<Uuid>>(&citations_text.0) {
            Ok(citations) => citations,
            Err(err) => {
                tracing::error!("Failed to parse citations JSON: {}", err);
                return Err(ErrorBadRequest(
                    "Invalid citations format. Expected JSON array of publication UUIDs",
                ));
            }
        },
        None => Vec::new(),
    };

    let file_name = form
        .file
        .file_name
        .as_ref()
        .map(|n| n.to_string())
        .unwrap_or_else(|| "unnamed.pdf".to_string());

    let publication_uuid = Uuid::new_v4();
    let s3_directory = format!("publications/{}", publication_uuid);
    let s3key = format!("{}/{}", s3_directory, file_name);

    let author = data.sql_client.get_author(&user_id).await.map_err(|err| {
        tracing::error!("Error creating publication: {}", err);
        ErrorInternalServerError("Internal server error")
    })?;

    let capability = generate_capability_for_publication(
        form.file.file.as_file(),
        form.price.0,
        &author.wallet_address,
    )
    .map_err(|err| {
        tracing::error!("Failed to generate capability: {}", err);
        err // Return the error to fail the request
    })?;

    data.s3_client
        .upload_storage_files(vec![form.file], Some(s3_directory.into()))
        .await
        .map_err(|err| {
            tracing::error!("Error uploading file to S3: {}", err);
            ErrorInternalServerError("Failed to upload file")
        })?;

    let new_publication = NewPublication {
        user_id: user_id.clone(),
        title: form.title.0,
        about: form.about.0,
        tags,
        s3key,
        price: form.price.0,
        citation_royalty_bps: form.citation_royalty_bps.0,
    };

    let publication = data
        .sql_client
        .create_publication(&new_publication)
        .await
        .map_err(|err| {
            tracing::error!("Error creating publication: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if let Err(err) = data
        .sql_client
        .set_publication_authors(publication.id, &authors)
        .await
    {
        tracing::error!(
            "Error setting authors for publication {}: {}",
            publication.id,
            err
        );
    }

    process_citations(&data.sql_client, publication.id, citations).await;

    // Fetch wallet addresses for all authors
    let author_wallet_addresses = fetch_author_wallet_addresses(&data.sql_client, &authors)
        .await
        .map_err(|err| {
            tracing::error!("Failed to fetch author wallet addresses: {}", err);
            ErrorInternalServerError("Failed to fetch author wallet addresses")
        })?;

    let response = serde_json::json!({
        "publication": publication,
        "capability": capability,
        "author_wallets": author_wallet_addresses,
    });

    Ok(HttpResponse::Ok().json(response))
}

fn generate_capability_for_publication(
    publication_file: &std::fs::File,
    price: i64,
    recipient: &str,
) -> Result<SignedCapability, actix_web::Error> {
    use aptos_sdk::types::account_address::AccountAddress;

    let paper_hash = hash_file_sha3_256(publication_file).unwrap();

    let capability_signer = CapabilitySigner::from_config(&CONFIG).map_err(|err| {
        tracing::error!("Failed to create capability signer: {}", err);
        ErrorInternalServerError("Internal server error")
    })?;

    // Parse recipient address
    let recipient_addr = AccountAddress::from_hex_literal(recipient).map_err(|err| {
        tracing::error!("Invalid recipient address {}: {}", recipient, err);
        ErrorBadRequest("Invalid recipient address")
    })?;

    capability_signer
        .create_capability(&paper_hash, price as u64, &recipient_addr, 3600)
        .map_err(|err| {
            tracing::error!("Failed to create capability: {}", err);
            ErrorInternalServerError("Failed to create capability")
        })
}

fn hash_file_sha3_256(file: &std::fs::File) -> Result<[u8; 32], std::io::Error> {
    let mut reader = BufReader::new(file);
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

#[get("/{publication_id}")]
async fn get_publication(
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
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
    userId: Option<Text<String>>, // Changed from Uuid to String
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
            .upload_storage_files(vec![file], Some(s3_directory.into()))
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
            form.userId.as_ref().map(|u| u.0.as_str()),
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

    let user_id = claims.sub;
    delete_publication_internal(&data, *publication_id, &user_id).await
}

/// Internal function to delete a publication (used for rollback)
async fn delete_publication_internal(
    data: &AppState,
    publication_id: Uuid,
    user_id: &str,
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
    if publication.user_id != user_id {
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

    if !publication.s3key.is_empty() {
        if let Some(file_name) = publication.s3key.split('/').last() {
            if let Err(err) = data
                .s3_client
                .delete_storage_files(vec![file_name.to_string()], None)
                .await
            {
                tracing::warn!("Failed to delete S3 file {}: {}", publication.s3key, err);
                // Continue with database deletion even if S3 deletion fails
            }
        }
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

#[derive(Deserialize)]
struct ListPublicationsQuery {
    page: Option<i64>,
    limit: Option<i64>,
}

#[get("/user/{privy_id}")]
async fn list_publications_by_user(
    privy_id: web::Path<String>,
    data: web::Data<AppState>,
    query: web::Query<ListPublicationsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let publications = data
        .sql_client
        .list_publications_by_user(&privy_id, query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error listing user publications: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    let total_count = data
        .sql_client
        .count_publications_by_user(&privy_id)
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

// TODO: remove endpoint
#[get("/{publication_id}/pdf-url")]
async fn get_publication_pdf_url(
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
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

/// Helper function to fetch wallet addresses for authors
async fn fetch_author_wallet_addresses(
    sql_client: &SqlClient,
    author_ids: &[PrivyId],
) -> Result<Vec<String>, actix_web::Error> {
    let mut wallet_addresses = Vec::new();

    for author_id in author_ids {
        match sql_client.get_author(author_id).await {
            Ok(author) => {
                wallet_addresses.push(author.wallet_address);
            }
            Err(err) => {
                tracing::error!("Failed to fetch author {}: {}", author_id, err);
                return Err(ErrorInternalServerError("Failed to fetch author details"));
            }
        }
    }

    Ok(wallet_addresses)
}

/// Helper function to process citations for a publication
async fn process_citations(sql_client: &SqlClient, publication_id: Uuid, citations: Vec<Uuid>) {
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
