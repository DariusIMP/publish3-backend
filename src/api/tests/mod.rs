// Test utilities for API endpoint testing
use std::sync::Arc;

use actix_web::{
    web::Data,
    App,
};
use aws_sdk_s3::config::Credentials;
use redis::Client;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::{
    db::{
        s3::client::S3Client,
        sql::SqlClient,
    },
    AppState,
};

// Test application setup
pub async fn create_test_app(pool: PgPool) -> App<impl actix_web::dev::ServiceFactory<actix_web::dev::ServiceRequest, Config = (), Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>, Error = actix_web::Error, InitError = ()>> {
    let sql_client = Arc::new(SqlClient::new(pool).await);
    
    // Create mock S3 credentials (won't actually be used in tests)
    let s3_credentials = Credentials::new(
        "test_access_key".to_string(),
        "test_secret_key".to_string(),
        None,
        None,
        "Test",
    );
    
    let s3_client = Arc::new(S3Client::new(s3_credentials, None, Some("http://localhost:9000".to_string())).await);
    
    // Create a mock Redis client
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

// Helper to create test user
pub async fn create_test_user(sql_client: &SqlClient) -> Uuid {
    use crate::db::sql::{models::NewUser, UserOperations};
    
    let new_user = NewUser {
        username: format!("testuser_{}", Uuid::new_v4()),
        email: format!("test_{}@example.com", Uuid::new_v4()),
        password_hash: "test_hash".to_string(),
        full_name: Some("Test User".to_string()),
        avatar_s3key: None,
        is_active: Some(true),
        is_admin: Some(false),
    };
    
    let user = sql_client.create_user(&new_user).await.unwrap();
    user.id
}

// Helper to create test author
pub async fn create_test_author(sql_client: &SqlClient) -> Uuid {
    use crate::db::sql::{models::NewAuthor, AuthorOperations};
    
    let new_author = NewAuthor {
        name: format!("Test Author {}", Uuid::new_v4()),
        email: Some(format!("author_{}@example.com", Uuid::new_v4())),
        affiliation: Some("Test University".to_string()),
    };
    
    let author = sql_client.create_author(&new_author).await.unwrap();
    author.id
}

// Helper to create test publication
pub async fn create_test_publication(sql_client: &SqlClient, user_id: Option<Uuid>) -> Uuid {
    use crate::db::sql::{models::NewPublication, PublicationOperations};
    
    let new_publication = NewPublication {
        user_id,
        title: format!("Test Publication {}", Uuid::new_v4()),
        about: Some("Test publication description".to_string()),
        tags: Some(vec!["test".to_string(), "research".to_string()]),
        s3key: None,
    };
    
    let publication = sql_client.create_publication(&new_publication).await.unwrap();
    publication.id
}

// Helper to create test citation
pub async fn create_test_citation(sql_client: &SqlClient, citing_id: Uuid, cited_id: Uuid) -> Uuid {
    use crate::db::sql::{models::NewCitation, CitationOperations};
    
    let new_citation = NewCitation {
        citing_publication_id: citing_id,
        cited_publication_id: cited_id,
        citation_context: Some("Test citation context".to_string()),
    };
    
    let citation = sql_client.create_citation(&new_citation).await.unwrap();
    citation.id
}

// Test database setup
pub async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/publish3_test".to_string());
    
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create test database pool")
}
