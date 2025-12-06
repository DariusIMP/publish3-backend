use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub full_name: Option<String>,
    pub avatar_s3key: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Author {
    pub id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub affiliation: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Publication {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub title: String,
    pub about: Option<String>,
    pub tags: Vec<String>,
    pub s3key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PublicationAuthor {
    pub publication_id: Uuid,
    pub author_id: Uuid,
    pub author_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Citation {
    pub id: Uuid,
    pub citing_publication_id: Uuid,
    pub cited_publication_id: Uuid,
    pub citation_context: Option<String>,
    pub created_at: DateTime<Utc>,
}

// New structs for creating/updating records (without auto-generated fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub full_name: Option<String>,
    pub avatar_s3key: Option<String>,
    pub is_active: Option<bool>,
    pub is_admin: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAuthor {
    pub name: String,
    pub email: Option<String>,
    pub affiliation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPublication {
    pub user_id: Option<Uuid>,
    pub title: String,
    pub about: Option<String>,
    pub tags: Option<Vec<String>>,
    pub s3key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPublicationAuthor {
    pub publication_id: Uuid,
    pub author_id: Uuid,
    pub author_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCitation {
    pub citing_publication_id: Uuid,
    pub cited_publication_id: Uuid,
    pub citation_context: Option<String>,
}
