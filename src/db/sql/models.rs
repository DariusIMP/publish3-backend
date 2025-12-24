use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::db::sql::PrivyId;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub privy_id: PrivyId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Author {
    pub privy_id: PrivyId,
    pub name: String,
    pub email: Option<String>,
    pub affiliation: Option<String>,
    pub wallet_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Publication {
    pub id: Uuid,
    pub user_id: PrivyId,
    pub title: String,
    pub about: String,
    pub tags: Vec<String>,
    pub s3key: String,
    pub price: i64,
    pub citation_royalty_bps: i64,
    pub status: String,
    pub transaction_hash: Option<String>,
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
pub struct PublicationAuthorWithDetails {
    pub publication_id: Uuid,
    pub author_id: PrivyId,
    pub author_order: i32,
    pub author_name: String,
    pub author_email: Option<String>,
    pub author_affiliation: Option<String>,
    pub author_wallet_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Wallet {
    pub wallet_id: String,
    pub wallet_address: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserWallet {
    pub user_id: String,
    pub wallet_id: String,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
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
    pub privy_id: PrivyId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAuthor {
    pub privy_id: PrivyId,
    pub name: String,
    pub email: Option<String>,
    pub affiliation: Option<String>,
    pub wallet_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPublication {
    pub user_id: PrivyId,
    pub title: String,
    pub about: String,
    pub tags: Vec<String>,
    pub s3key: String,
    pub price: i64,
    pub citation_royalty_bps: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPublicationAuthor {
    pub publication_id: Uuid,
    pub author_id: String,
    pub author_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWallet {
    pub wallet_id: String,
    pub wallet_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUserWallet {
    pub user_id: String,
    pub wallet_id: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCitation {
    pub citing_publication_id: Uuid,
    pub cited_publication_id: Uuid,
}
