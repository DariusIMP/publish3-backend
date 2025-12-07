use actix_web::{
    HttpResponse, delete,
    error::{ErrorBadRequest, ErrorConflict, ErrorInternalServerError, ErrorNotFound},
    get, post, put, web,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    db::sql::{models::NewCitation, CitationOperations},
};

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/citations")
        .service(create_citation)
        .service(get_citation)
        .service(update_citation)
        .service(delete_citation)
        .service(list_citations)
        .service(get_citation_by_publications)
        .service(count_citations);
    conf.service(scope);
}

#[derive(Deserialize)]
pub struct CreateCitationRequest {
    citing_publication_id: Uuid,
    cited_publication_id: Uuid,
    citation_context: Option<String>,
}

#[post("/create")]
async fn create_citation(
    request: web::Json<CreateCitationRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Check if citation already exists between these publications
    let existing_citation = data
        .sql_client
        .get_citation_by_publications(request.citing_publication_id, request.cited_publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error checking existing citation: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if existing_citation.is_some() {
        return Err(ErrorConflict("Citation already exists between these publications"));
    }

    // Check that publications are not the same
    if request.citing_publication_id == request.cited_publication_id {
        return Err(ErrorBadRequest("A publication cannot cite itself"));
    }

    let new_citation = NewCitation {
        citing_publication_id: request.citing_publication_id,
        cited_publication_id: request.cited_publication_id,
        citation_context: request.citation_context.clone(),
    };

    let citation = data
        .sql_client
        .create_citation(&new_citation)
        .await
        .map_err(|err| {
            tracing::error!("Error creating citation: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(citation))
}

#[get("/{citation_id}")]
async fn get_citation(
    citation_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let citation = data
        .sql_client
        .get_citation(*citation_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving citation: {}", err);
            match err {
                sqlx::Error::RowNotFound => ErrorNotFound("Citation not found"),
                _ => ErrorInternalServerError("Internal server error"),
            }
        })?;

    Ok(HttpResponse::Ok().json(citation))
}

#[derive(Deserialize)]
pub struct UpdateCitationRequest {
    citation_context: Option<String>,
}

#[put("/{citation_id}")]
async fn update_citation(
    citation_id: web::Path<Uuid>,
    request: web::Json<UpdateCitationRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let result = data
        .sql_client
        .update_citation(*citation_id, request.citation_context.as_deref())
        .await
        .map_err(|err| {
            tracing::error!("Error updating citation: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if result.rows_affected() == 0 {
        return Err(ErrorNotFound("Citation not found"));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Citation updated successfully"
    })))
}

#[delete("/{citation_id}")]
async fn delete_citation(
    citation_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let result = data
        .sql_client
        .delete_citation(*citation_id)
        .await
        .map_err(|err| {
            tracing::error!("Error deleting citation: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if result.rows_affected() == 0 {
        return Err(ErrorNotFound("Citation not found"));
    }

    Ok(HttpResponse::NoContent().finish())
}

#[get("/list")]
async fn list_citations(
    data: web::Data<AppState>,
    query: web::Query<ListCitationsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let citations = data
        .sql_client
        .list_citations(query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error listing citations: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    let total_count = data
        .sql_client
        .count_citations()
        .await
        .map_err(|err| {
            tracing::error!("Error counting citations: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "citations": citations,
        "total": total_count,
        "page": query.page.unwrap_or(1),
        "limit": query.limit.unwrap_or(20)
    })))
}

#[derive(Deserialize)]
struct ListCitationsQuery {
    page: Option<i64>,
    limit: Option<i64>,
}

#[get("/by-publications")]
async fn get_citation_by_publications(
    data: web::Data<AppState>,
    query: web::Query<CitationByPublicationsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let citation = data
        .sql_client
        .get_citation_by_publications(query.citing_publication_id, query.cited_publication_id)
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving citation by publications: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    match citation {
        Some(citation) => Ok(HttpResponse::Ok().json(citation)),
        None => Err(ErrorNotFound("Citation not found between these publications")),
    }
}

#[derive(Deserialize)]
struct CitationByPublicationsQuery {
    citing_publication_id: Uuid,
    cited_publication_id: Uuid,
}

#[get("/count")]
async fn count_citations(
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let count = data
        .sql_client
        .count_citations()
        .await
        .map_err(|err| {
            tracing::error!("Error counting citations: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "count": count
    })))
}
