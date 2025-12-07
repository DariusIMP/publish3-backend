use actix_multipart::form::{MultipartForm, tempfile::TempFile, text::Text};
use actix_web::{
    HttpResponse, delete,
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound},
    get, post, put, web,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    db::sql::{models::NewPublication, PublicationOperations},
};

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/publications")
        .service(create_publication)
        .service(get_publication)
        .service(update_publication)
        .service(delete_publication)
        .service(list_publications)
        .service(list_publications_by_user)
        .service(search_publications_by_title)
        .service(search_publications_by_tag)
        .service(get_publication_authors)
        .service(get_publication_citations)
        .service(get_cited_by)
        .service(count_publications)
        .service(count_publications_by_user);
    conf.service(scope);
}

#[derive(MultipartForm)]
#[allow(non_snake_case)]
pub struct CreatePublicationForm {
    userId: Option<Text<Uuid>>,
    title: Text<String>,
    about: Option<Text<String>>,
    tags: Option<Text<String>>, // JSON array string like ["tag1", "tag2"]
    file: Option<TempFile>,
}

#[post("/create")]
async fn create_publication(
    MultipartForm(form): MultipartForm<CreatePublicationForm>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Parse tags from JSON array string
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
        
        let s3_path = format!("publications/{}/{}", Uuid::new_v4(), file_name);
        
        // Use upload_storage_files which expects Vec<TempFile>
        data.s3_client
            .upload_storage_files(vec![file], Some(s3_path.clone().into()))
            .await
            .map_err(|err| {
                tracing::error!("Error uploading file to S3: {}", err);
                ErrorInternalServerError("Failed to upload file")
            })?;
        
        s3key = Some(s3_path);
    }

    let new_publication = NewPublication {
        user_id: form.userId.map(|u| u.0),
        title: form.title.0,
        about: form.about.map(|a| a.0),
        tags,
        s3key,
    };

    let publication = data
        .sql_client
        .create_publication(&new_publication)
        .await
        .map_err(|err| {
            tracing::error!("Error creating publication: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(publication))
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

    Ok(HttpResponse::Ok().json(publication))
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
        
        let s3_path = format!("publications/{}/{}", Uuid::new_v4(), file_name);
        
        // Use upload_storage_files which expects Vec<TempFile>
        data.s3_client
            .upload_storage_files(vec![file], Some(s3_path.clone().into()))
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
            form.userId.map(|u| u.0),
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
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // First get the publication to check if it has an S3 file
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

    // Delete S3 file if it exists
    if let Some(s3key) = &publication.s3key {
        // Extract just the filename from the path for deletion
        if let Some(file_name) = s3key.split('/').last() {
            if let Err(err) = data.s3_client.delete_storage_files(vec![file_name.to_string()], None).await {
                tracing::warn!("Failed to delete S3 file {}: {}", s3key, err);
                // Continue with database deletion even if S3 deletion fails
            }
        }
    }

    let result = data
        .sql_client
        .delete_publication(*publication_id)
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
    let publications = data
        .sql_client
        .list_publications(query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error listing publications: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    let total_count = data
        .sql_client
        .count_publications()
        .await
        .map_err(|err| {
            tracing::error!("Error counting publications: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

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

#[get("/user/{user_id}")]
async fn list_publications_by_user(
    user_id: web::Path<Uuid>,
    data: web::Data<AppState>,
    query: web::Query<ListPublicationsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let publications = data
        .sql_client
        .list_publications_by_user(*user_id, query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error listing user publications: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    let total_count = data
        .sql_client
        .count_publications_by_user(*user_id)
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
async fn get_publication_authors(
    publication_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let authors = data
        .sql_client
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

#[get("/count")]
async fn count_publications(
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let count = data
        .sql_client
        .count_publications()
        .await
        .map_err(|err| {
            tracing::error!("Error counting publications: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "count": count
    })))
}

#[get("/count/user/{user_id}")]
async fn count_publications_by_user(
    user_id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let count = data
        .sql_client
        .count_publications_by_user(*user_id)
        .await
        .map_err(|err| {
            tracing::error!("Error counting user publications: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "count": count
    })))
}
