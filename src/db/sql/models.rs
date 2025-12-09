use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::db::sql::PrivyId;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub privy_id: PrivyId,
    pub username: String,
    pub email: String,
    pub full_name: Option<String>,
    pub avatar_s3key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Author {
    pub privy_id: PrivyId,
    pub name: String,
    pub email: Option<String>,
    pub affiliation: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Publication {
    pub id: Uuid,
    pub user_id: Option<PrivyId>,
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
    pub author_id: PrivyId, // Now references authors(privy_id) as VARCHAR
    pub author_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Citation {
    pub id: Uuid,
    pub citing_publication_id: Uuid,
    pub cited_publication_id: Uuid,
    pub created_at: DateTime<Utc>,
}

// New structs for creating/updating records (without auto-generated fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub full_name: Option<String>,
    pub avatar_s3key: Option<String>,
    pub privy_id: PrivyId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAuthor {
    pub privy_id: PrivyId,
    pub name: String,
    pub email: Option<String>,
    pub affiliation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPublication {
    pub user_id: PrivyId,
    pub title: String,
    pub about: Option<String>,
    pub tags: Option<Vec<String>>,
    pub s3key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPublicationAuthor {
    pub publication_id: Uuid,
    pub author_id: String,
    pub author_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCitation {
    pub citing_publication_id: Uuid,
    pub cited_publication_id: Uuid,
}
