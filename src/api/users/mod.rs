use actix_multipart::form::{MultipartForm, tempfile::TempFile, text::Text};
use actix_web::{
    HttpRequest, HttpResponse, delete,
    error::{ErrorConflict, ErrorInternalServerError, ErrorNotFound},
    get, post, put, web,
};
use uuid::Uuid;

use crate::{
    AppState,
    db::sql::{
        AuthorOperations, PrivyId, UserOperations,
        models::{NewAuthor, NewUser},
    },
};

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/users")
        .service(create_user)
        .service(get_user)
        .service(update_user)
        .service(delete_user)
        .service(list_users)
        .service(get_user_avatar)
        .service(sign_in);
    conf.service(scope);
}

#[derive(MultipartForm)]
#[allow(non_snake_case)]
pub struct CreateUserForm {
    username: Text<String>,
    email: Text<String>,
    fullName: Option<Text<String>>,
    avatar: Option<TempFile>,
    privy_id: Text<String>,
    // Removed: isActive, isAdmin
}

#[post("/create")]
async fn create_user(
    MultipartForm(form): MultipartForm<CreateUserForm>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Check if user with email already exists
    let email_exists = data
        .sql_client
        .user_email_exists(&form.email.0)
        .await
        .map_err(|_| ErrorInternalServerError("Internal server error"))?;

    if email_exists {
        return Err(ErrorConflict("User with that email already exists"));
    }

    // Check if user with username already exists
    let username_exists = data
        .sql_client
        .user_username_exists(&form.username.0)
        .await
        .map_err(|_| ErrorInternalServerError("Internal server error"))?;

    if username_exists {
        return Err(ErrorConflict("User with that username already exists"));
    }

    // Check if user with privy_id already exists
    let user_by_privy_id = data
        .sql_client
        .get_user_by_privy_id(form.privy_id.0.clone())
        .await;
    if user_by_privy_id.is_ok() {
        return Err(ErrorConflict("User with that privy_id already exists"));
    }

    // Handle avatar upload if present
    let mut avatar_s3key = None;
    if let Some(_avatar) = form.avatar {
        // TODO: Implement S3 upload for user avatars
        // For now, we'll just store a placeholder
        avatar_s3key = Some(format!("avatars/{}", Uuid::new_v4()));
    }

    let new_user = NewUser {
        username: form.username.0,
        email: form.email.0,
        full_name: form.fullName.map(|f| f.0),
        avatar_s3key,
        privy_id: form.privy_id.0,
    };

    let user = data
        .sql_client
        .create_user(&new_user)
        .await
        .map_err(|err| {
            tracing::error!("Error creating user: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    Ok(HttpResponse::Ok().json(user))
}

#[get("/{privy_id}")]
async fn get_user(
    privy_id: web::Path<PrivyId>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let user = data
        .sql_client
        .get_user(privy_id.to_string())
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving user: {}", err);
            match err {
                sqlx::Error::RowNotFound => ErrorNotFound("User not found"),
                _ => ErrorInternalServerError("Internal server error"),
            }
        })?;

    Ok(HttpResponse::Ok().json(user))
}

#[derive(MultipartForm)]
#[allow(non_snake_case)]
pub struct UpdateUserForm {
    username: Option<Text<String>>,
    email: Option<Text<String>>,
    fullName: Option<Text<String>>,
    avatar: Option<TempFile>,
}

#[put("/{privy_id}")]
async fn update_user(
    privy_id: web::Path<PrivyId>,
    MultipartForm(form): MultipartForm<UpdateUserForm>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    // Handle avatar upload if present
    let mut avatar_s3key = None;
    if let Some(_avatar) = form.avatar {
        // TODO: Implement S3 upload for user avatars
        // For now, we'll just store a placeholder
        avatar_s3key = Some(format!("avatars/{}", Uuid::new_v4()));
    }

    let result = data
        .sql_client
        .update_user(
            privy_id.to_string(),
            form.username.as_ref().map(|u| u.0.as_str()),
            form.email.as_ref().map(|e| e.0.as_str()),
            form.fullName.as_ref().map(|f| f.0.as_str()),
            avatar_s3key.as_deref(),
        )
        .await
        .map_err(|err| {
            tracing::error!("Error updating user: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if result.rows_affected() == 0 {
        return Err(ErrorNotFound("User not found"));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "User updated successfully"
    })))
}

#[delete("/{privy_id}")]
async fn delete_user(
    privy_id: web::Path<PrivyId>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let result = data
        .sql_client
        .delete_user(privy_id.to_string())
        .await
        .map_err(|err| {
            tracing::error!("Error deleting user: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    if result.rows_affected() == 0 {
        return Err(ErrorNotFound("User not found"));
    }

    Ok(HttpResponse::NoContent().finish())
}

#[get("/list")]
async fn list_users(
    data: web::Data<AppState>,
    query: web::Query<ListUsersQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let users = data
        .sql_client
        .list_users(query.page, query.limit)
        .await
        .map_err(|err| {
            tracing::error!("Error listing users: {}", err);
            ErrorInternalServerError("Internal server error")
        })?;

    let total_count = data.sql_client.count_users().await.map_err(|err| {
        tracing::error!("Error counting users: {}", err);
        ErrorInternalServerError("Internal server error")
    })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "users": users,
        "total": total_count,
        "page": query.page.unwrap_or(1),
        "limit": query.limit.unwrap_or(20)
    })))
}

#[derive(serde::Deserialize)]
struct ListUsersQuery {
    page: Option<i64>,
    limit: Option<i64>,
}

#[get("/avatar/{privy_id}")]
async fn get_user_avatar(
    privy_id: web::Path<PrivyId>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let user = data
        .sql_client
        .get_user(privy_id.to_string())
        .await
        .map_err(|err| {
            tracing::error!("Error retrieving user: {}", err);
            match err {
                sqlx::Error::RowNotFound => ErrorNotFound("User not found"),
                _ => ErrorInternalServerError("Internal server error"),
            }
        })?;

    // TODO: Implement S3 file retrieval for user avatars
    // For now, return a placeholder or no content
    match user.avatar_s3key {
        Some(_) => {
            // In a real implementation, we would fetch the file from S3
            // For now, return a placeholder response
            Ok(HttpResponse::Ok()
                .content_type("image/jpeg")
                .body("Placeholder for user avatar"))
        }
        None => Ok(HttpResponse::NoContent().finish()),
    }
}

#[post("/privy/sign-in")]
async fn sign_in(
    req: HttpRequest,
    data: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let claims = crate::auth::privy::get_privy_claims(&req).ok_or_else(|| {
        actix_web::error::ErrorUnauthorized("Valid Privy authentication token required")
    })?;

    let privy_id = claims.sub;

    let existing_user = data.sql_client.get_user_by_privy_id(privy_id.clone()).await;

    match existing_user {
        Ok(user) => {
            let existing_author = data.sql_client.get_author(&privy_id).await;

            let response = serde_json::json!({
                "user": user,
                "author": existing_author.ok(),
                "is_new_user": false,
            });

            Ok(HttpResponse::Ok().json(response))
        }
        Err(sqlx::Error::RowNotFound) => {
            // User doesn't exist, create new user and author

            let username = format!("user_{}", &privy_id[..10].to_lowercase());
            let email = format!("{}@privy.user", &privy_id[..10].to_lowercase());
            let full_name = "Privy User".to_string();

            let new_user = NewUser {
                username: username.clone(),
                email: email.clone(),
                full_name: Some(full_name.clone()),
                avatar_s3key: None,
                privy_id: privy_id.clone(),
            };

            let user = data
                .sql_client
                .create_user(&new_user)
                .await
                .map_err(|err| {
                    tracing::error!("Error creating user: {}", err);
                    ErrorInternalServerError("Failed to create user")
                })?;

            let new_author = NewAuthor {
                privy_id: privy_id.clone(),
                name: full_name,
                email: Some(email),
                affiliation: None,
            };

            let author = data
                .sql_client
                .create_author(&new_author)
                .await
                .map_err(|err| {
                    tracing::error!("Error creating author: {}", err);
                    ErrorInternalServerError("Failed to create author")
                })?;

            let response = serde_json::json!({
                "user": user,
                "author": author,
                "is_new_user": true,
            });

            Ok(HttpResponse::Created().json(response))
        }
        Err(err) => {
            tracing::error!("Error checking user existence: {}", err);
            Err(ErrorInternalServerError("Internal server error"))
        }
    }
}
