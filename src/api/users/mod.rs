use actix_web::{
    HttpRequest, HttpResponse, delete,
    error::{ErrorConflict, ErrorInternalServerError, ErrorNotFound},
    get, post, web,
};

use crate::{
    AppState,
    db::sql::{AuthorOperations, PrivyId, UserOperations, models::NewUser},
};

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/users")
        .service(create_user)
        .service(get_user)
        .service(delete_user)
        .service(list_users)
        .service(sign_in);
    conf.service(scope);
}

#[post("/create")]
async fn create_user(
    data: web::Data<AppState>,
    body: web::Json<CreateUserRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    // Check if user with privy_id already exists
    let user_by_privy_id = data
        .sql_client
        .get_user_by_privy_id(body.privy_id.clone())
        .await;

    if user_by_privy_id.is_ok() {
        return Err(ErrorConflict("User with that privy_id already exists"));
    }

    let new_user = NewUser {
        privy_id: body.privy_id.clone(),
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

#[derive(serde::Deserialize)]
struct CreateUserRequest {
    privy_id: PrivyId,
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

    let author = data.sql_client.get_author(&privy_id).await.ok();

    let response = serde_json::json!({
        "user": user,
        "author": author,
    });

    Ok(HttpResponse::Ok().json(response))
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
            });

            Ok(HttpResponse::Ok().json(response))
        }
        Err(sqlx::Error::RowNotFound) => {
            let new_user = NewUser {
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

            let response = serde_json::json!({
                "user": user,
            });

            Ok(HttpResponse::Created().json(response))
        }
        Err(err) => {
            tracing::error!("Error checking user existence: {}", err);
            Err(ErrorInternalServerError("Internal server error"))
        }
    }
}
