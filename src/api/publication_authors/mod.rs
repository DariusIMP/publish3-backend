use actix_web::{
    HttpResponse, delete,
    error::{ErrorBadRequest, ErrorConflict, ErrorInternalServerError, ErrorNotFound},
    get, post, put, web,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    db::sql::{PrivyId, PublicationAuthorOperations},
};

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/publication-authors")
        .service(add_author_to_publication)
        .service(remove_author_from_publication)
        .service(set_publication_authors)
        .service(update_author_order)
        .service(get_publication_authors)
        .service(publication_has_author)
        .service(count_authors_for_publication)
        .service(get_author_publications)
        .service(count_publications_for_author);
    conf.service(scope);
}

#[derive(Deserialize)]
pub struct AddAuthorToPublicationRequest {
    publication_id: Uuid,
    author_id: PrivyId,
    author_order: Option<i32>,
}

#[post("/add")]
async fn add_author_to_publication(
    request: web::Json<AddAuthorToPublicationRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Check if author is already associated with publication
    let has_author = data
        .sql_client
        .publication_has_author(request.publication_id, &request.author_id)
        .await
        .map_err(|err| {
            tracing::error!("Error checking author association: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if has_author {
        return Err(ErrorConflict("Author is already associated with this publication"));
    }

    // Note: We need to use the PublicationAuthorOperations trait method
    data.sql_client
        .add_author_to_publication(request.publication_id, &request.author_id, request.author_order)
        .await
        .map_err(|err| {
            tracing::error!("Error adding author to publication: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Author added to publication successfully"
    })))
}

#[derive(Deserialize)]
pub struct RemoveAuthorFromPublicationRequest {
    publication_id: Uuid,
    author_id: PrivyId,
}

#[delete("/remove")]
async fn remove_author_from_publication(
    request: web::Json<RemoveAuthorFromPublicationRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let result = data
        .sql_client
        .remove_author_from_publication(request.publication_id, &request.author_id)
        .await
        .map_err(|err| {
            tracing::error!("Error removing author from publication: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if result.rows_affected() == 0 {
        return Err(ErrorNotFound("Author not found in publication"));
    }

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize)]
pub struct SetPublicationAuthorsRequest {
    publication_id: Uuid,
    author_ids: Vec<PrivyId>,
}

#[post("/set")]
async fn set_publication_authors(
    request: web::Json<SetPublicationAuthorsRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Check for duplicate author IDs
    let unique_author_ids: Vec<PrivyId> = request.author_ids
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    if unique_author_ids.len() != request.author_ids.len() {
        return Err(ErrorBadRequest("Duplicate author IDs are not allowed"));
    }

    data.sql_client
        .set_publication_authors(request.publication_id, &request.author_ids)
        .await
        .map_err(|err| {
            tracing::error!("Error setting publication authors: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Publication authors set successfully"
    })))
}

#[derive(Deserialize)]
pub struct UpdateAuthorOrderRequest {
    publication_id: Uuid,
    author_id: PrivyId,
    author_order: i32,
}

#[put("/order")]
async fn update_author_order(
    request: web::Json<UpdateAuthorOrderRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let result = data
        .sql_client
        .update_author_order(request.publication_id, &request.author_id, request.author_order)
        .await
        .map_err(|err| {
            tracing::error!("Error updating author order: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if result.rows_affected() == 0 {
        return Err(ErrorNotFound("Author not found in publication"));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Author order updated successfully"
    })))
}

#[get("/publication/{publication_id}")]
async fn get_publication_authors(
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let authors = data.sql_client
        .get_publication_authors(*publication_id)
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

#[get("/has-author")]
async fn publication_has_author(
    data: web::Data<AppState>,
    query: web::Query<HasAuthorQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let has_author = data
        .sql_client
        .publication_has_author(query.publication_id, &query.author_id)
        .await
        .map_err(|err| {
            tracing::error!("Error checking author association: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "has_author": has_author
    })))
}

#[derive(Deserialize)]
struct HasAuthorQuery {
    publication_id: Uuid,
    author_id: PrivyId,
}

#[get("/count/{publication_id}")]
async fn count_authors_for_publication(
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let count = data
        .sql_client
        .count_authors_for_publication(*publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error counting authors for publication: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "count": count
    })))
}

#[get("/author/{author_id}")]
async fn get_author_publications(
    author_id: web::Path<PrivyId>,
    data: web::Data<AppState>,
    query: web::Query<AuthorPublicationsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let publications = data
        .sql_client
        .get_author_publications(&author_id, query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving author publications: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(publications))
}

#[derive(Deserialize)]
struct AuthorPublicationsQuery {
    page: Option<i64>,
    limit: Option<i64>,
}

#[get("/count/author/{author_id}")]
async fn count_publications_for_author(
    author_id: web::Path<PrivyId>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let count = data
        .sql_client
        .count_publications_for_author(&author_id)
        .await
        .map_err(|err| {
            tracing::error!("Error counting publications for author: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "count": count
    })))
}
