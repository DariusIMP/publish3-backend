use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgQueryResult, FromRow};
use uuid::Uuid;

use crate::db::sql::{models::User, SqlClient};

#[async_trait]
pub trait UserOperations {
    async fn create_user(&self, new_user: &super::models::NewUser) -> Result<User, sqlx::Error>;
    
    async fn get_user(&self, user_id: Uuid) -> Result<User, sqlx::Error>;
    
    async fn get_user_by_email(&self, email: &str) -> Result<User, sqlx::Error>;
    
    async fn get_user_by_username(&self, username: &str) -> Result<User, sqlx::Error>;
    
    async fn list_users(
        &self, 
        page: Option<i64>, 
        limit: Option<i64>
    ) -> Result<Vec<User>, sqlx::Error>;
    
    async fn update_user(
        &self,
        user_id: Uuid,
        username: Option<&str>,
        email: Option<&str>,
        password_hash: Option<&str>,
        full_name: Option<&str>,
        avatar_s3key: Option<&str>,
        is_active: Option<bool>,
        is_admin: Option<bool>,
    ) -> Result<PgQueryResult, sqlx::Error>;
    
    async fn delete_user(&self, user_id: Uuid) -> Result<PgQueryResult, sqlx::Error>;
    
    async fn user_email_exists(&self, email: &str) -> Result<bool, sqlx::Error>;
    
    async fn user_username_exists(&self, username: &str) -> Result<bool, sqlx::Error>;
    
    async fn count_users(&self) -> Result<i64, sqlx::Error>;
}

#[async_trait]
impl UserOperations for SqlClient {
    async fn create_user(&self, new_user: &super::models::NewUser) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users 
            (username, email, password_hash, full_name, avatar_s3key, is_active, is_admin)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, username, email, password_hash, full_name, avatar_s3key, 
                      is_active, is_admin, created_at, updated_at
            "#,
        )
        .bind(&new_user.username)
        .bind(&new_user.email)
        .bind(&new_user.password_hash)
        .bind(&new_user.full_name)
        .bind(&new_user.avatar_s3key)
        .bind(new_user.is_active.unwrap_or(true))
        .bind(new_user.is_admin.unwrap_or(false))
        .fetch_one(&self.db)
        .await
    }
    
    async fn get_user(&self, user_id: Uuid) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, password_hash, full_name, avatar_s3key, 
                   is_active, is_admin, created_at, updated_at
            FROM users 
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await
    }
    
    async fn get_user_by_email(&self, email: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, password_hash, full_name, avatar_s3key, 
                   is_active, is_admin, created_at, updated_at
            FROM users 
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_one(&self.db)
        .await
    }
    
    async fn get_user_by_username(&self, username: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, password_hash, full_name, avatar_s3key, 
                   is_active, is_admin, created_at, updated_at
            FROM users 
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_one(&self.db)
        .await
    }
    
    async fn list_users(
        &self, 
        page: Option<i64>, 
        limit: Option<i64>
    ) -> Result<Vec<User>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;
        
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, password_hash, full_name, avatar_s3key, 
                   is_active, is_admin, created_at, updated_at
            FROM users 
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }
    
    async fn update_user(
        &self,
        user_id: Uuid,
        username: Option<&str>,
        email: Option<&str>,
        password_hash: Option<&str>,
        full_name: Option<&str>,
        avatar_s3key: Option<&str>,
        is_active: Option<bool>,
        is_admin: Option<bool>,
    ) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users SET
            username = COALESCE($1, username),
            email = COALESCE($2, email),
            password_hash = COALESCE($3, password_hash),
            full_name = COALESCE($4, full_name),
            avatar_s3key = COALESCE($5, avatar_s3key),
            is_active = COALESCE($6, is_active),
            is_admin = COALESCE($7, is_admin),
            updated_at = NOW()
            WHERE id = $8
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(full_name)
        .bind(avatar_s3key)
        .bind(is_active)
        .bind(is_admin)
        .bind(user_id)
        .execute(&self.db)
        .await
    }
    
    async fn delete_user(&self, user_id: Uuid) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            "DELETE FROM users WHERE id = $1",
        )
        .bind(user_id)
        .execute(&self.db)
        .await
    }
    
    async fn user_email_exists(&self, email: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)",
        )
        .bind(email)
        .fetch_one(&self.db)
        .await
    }
    
    async fn user_username_exists(&self, username: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)",
        )
        .bind(username)
        .fetch_one(&self.db)
        .await
    }
    
    async fn count_users(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM users",
        )
        .fetch_one(&self.db)
        .await
    }
}
