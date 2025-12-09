// Test utilities for API endpoint testing
use std::sync::Arc;

use actix_web::{App, web::Data};
use aws_sdk_s3::config::Credentials;
use redis::Client;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::{
    AppState,
    db::{s3::client::S3Client, sql::SqlClient},
};

pub async fn create_test_app(
    pool: PgPool,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let sql_client = Arc::new(SqlClient::new(pool).await);

    let s3_credentials = Credentials::new(
        "test_access_key".to_string(),
        "test_secret_key".to_string(),
        None,
        None,
        "Test",
    );

    let s3_client = Arc::new(
        S3Client::new(
            s3_credentials,
            None,
            Some("http://localhost:9000".to_string()),
        )
        .await,
    );

    let redis_client = Client::open("redis://localhost:6379").unwrap();

    let app_state = Data::new(AppState {
        sql_client,
        redis_client,
        s3_client,
    });

    App::new()
        .app_data(app_state.clone())
        .configure(crate::api::config)
}

pub async fn create_test_user(sql_client: &SqlClient) -> String {
    use crate::db::sql::{UserOperations, models::NewUser};

    let privy_id = format!("privy_test_user_{}", Uuid::new_v4());
    let new_user = NewUser {
        username: format!("testuser_{}", Uuid::new_v4()),
        email: format!("test_{}@example.com", Uuid::new_v4()),
        full_name: Some("Test User".to_string()),
        avatar_s3key: None,
        privy_id: privy_id.clone(),
    };

    let user = sql_client.create_user(&new_user).await.unwrap();
    user.privy_id
}

pub async fn create_test_author(sql_client: &SqlClient, user_privy_id: &str) -> String {
    use crate::db::sql::{AuthorOperations, models::NewAuthor};

    let new_author = NewAuthor {
        privy_id: user_privy_id.to_string(),
        name: format!("Test Author {}", Uuid::new_v4()),
        email: Some(format!("author_{}@example.com", Uuid::new_v4())),
        affiliation: Some("Test University".to_string()),
    };

    let author = sql_client.create_author(&new_author).await.unwrap();
    author.privy_id
}

pub async fn create_test_publication(sql_client: &SqlClient, user_privy_id: String) -> Uuid {
    use crate::db::sql::{PublicationOperations, models::NewPublication};

    let new_publication = NewPublication {
        user_id: user_privy_id,
        title: format!("Test Publication {}", Uuid::new_v4()),
        about: Some("Test publication description".to_string()),
        tags: Some(vec!["test".to_string(), "research".to_string()]),
        s3key: None,
    };

    let publication = sql_client
        .create_publication(&new_publication)
        .await
        .unwrap();
    publication.id
}

pub async fn create_test_citation(sql_client: &SqlClient, citing_id: Uuid, cited_id: Uuid) -> Uuid {
    use crate::db::sql::{CitationOperations, models::NewCitation};

    let new_citation = NewCitation {
        citing_publication_id: citing_id,
        cited_publication_id: cited_id,
    };

    let citation = sql_client.create_citation(&new_citation).await.unwrap();
    citation.id
}

pub async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost:5432/publish3_test".to_string()
    });

    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create test database pool")
}
