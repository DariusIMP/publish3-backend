use async_trait::async_trait;
use sqlx::postgres::PgQueryResult;

use crate::db::sql::{PrivyId, SqlClient, models::User};

#[async_trait]
pub trait UserOperations {
    async fn create_user(&self, new_user: &super::models::NewUser) -> Result<User, sqlx::Error>;

    async fn get_user(&self, privy_id: PrivyId) -> Result<User, sqlx::Error>;

    async fn get_user_by_email(&self, email: &str) -> Result<User, sqlx::Error>;

    async fn get_user_by_username(&self, username: &str) -> Result<User, sqlx::Error>;

    async fn list_users(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<User>, sqlx::Error>;

    async fn update_user(
        &self,
        privy_id: PrivyId,
        username: Option<&str>,
        email: Option<&str>,
        full_name: Option<&str>,
        avatar_s3key: Option<&str>,
    ) -> Result<PgQueryResult, sqlx::Error>;

    async fn delete_user(&self, privy_id: PrivyId) -> Result<PgQueryResult, sqlx::Error>;

    async fn user_email_exists(&self, email: &str) -> Result<bool, sqlx::Error>;

    async fn user_username_exists(&self, username: &str) -> Result<bool, sqlx::Error>;

    async fn count_users(&self) -> Result<i64, sqlx::Error>;

    async fn get_user_by_privy_id(&self, privy_id: PrivyId) -> Result<User, sqlx::Error>;
}

#[async_trait]
impl UserOperations for SqlClient {
    async fn create_user(&self, new_user: &super::models::NewUser) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users 
            (username, email, full_name, avatar_s3key, privy_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING privy_id, username, email, full_name, avatar_s3key, 
                      created_at, updated_at
            "#,
        )
        .bind(&new_user.username)
        .bind(&new_user.email)
        .bind(&new_user.full_name)
        .bind(&new_user.avatar_s3key)
        .bind(&new_user.privy_id)
        .fetch_one(&self.db)
        .await
    }

    async fn get_user(&self, privy_id: PrivyId) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT privy_id, username, email, full_name, avatar_s3key, 
                   created_at, updated_at
            FROM users 
            WHERE privy_id = $1
            "#,
        )
        .bind(privy_id)
        .fetch_one(&self.db)
        .await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT privy_id, username, email, full_name, avatar_s3key, 
                   created_at, updated_at
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
            SELECT privy_id, username, email, full_name, avatar_s3key, 
                   created_at, updated_at
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
        limit: Option<i64>,
    ) -> Result<Vec<User>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        sqlx::query_as::<_, User>(
            r#"
            SELECT privy_id, username, email, full_name, avatar_s3key, 
                   created_at, updated_at
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
        privy_id: PrivyId,
        username: Option<&str>,
        email: Option<&str>,
        full_name: Option<&str>,
        avatar_s3key: Option<&str>,
    ) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users SET
            username = COALESCE($1, username),
            email = COALESCE($2, email),
            full_name = COALESCE($3, full_name),
            avatar_s3key = COALESCE($4, avatar_s3key),
            updated_at = NOW()
            WHERE privy_id = $5
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(full_name)
        .bind(avatar_s3key)
        .bind(privy_id)
        .execute(&self.db)
        .await
    }

    async fn delete_user(&self, privy_id: PrivyId) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE privy_id = $1")
            .bind(privy_id)
            .execute(&self.db)
            .await
    }

    async fn user_email_exists(&self, email: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
            .bind(email)
            .fetch_one(&self.db)
            .await
    }

    async fn user_username_exists(&self, username: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
            .bind(username)
            .fetch_one(&self.db)
            .await
    }

    async fn count_users(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.db)
            .await
    }

    async fn get_user_by_privy_id(&self, privy_id: PrivyId) -> Result<User, sqlx::Error> {
        // This is now the same as get_user, but we keep it for API compatibility
        self.get_user(privy_id).await
    }
}
