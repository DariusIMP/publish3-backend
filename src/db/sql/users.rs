use async_trait::async_trait;
use sqlx::postgres::PgQueryResult;

use crate::db::sql::{PrivyId, SqlClient, models::User};

#[async_trait]
pub trait UserOperations {
    async fn create_user(&self, new_user: &super::models::NewUser) -> Result<User, sqlx::Error>;

    async fn get_user(&self, privy_id: PrivyId) -> Result<User, sqlx::Error>;

    async fn list_users(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<User>, sqlx::Error>;

    async fn delete_user(&self, privy_id: PrivyId) -> Result<PgQueryResult, sqlx::Error>;

    async fn count_users(&self) -> Result<i64, sqlx::Error>;

    async fn get_user_by_privy_id(&self, privy_id: PrivyId) -> Result<User, sqlx::Error>;
}

#[async_trait]
impl UserOperations for SqlClient {
    async fn create_user(&self, new_user: &super::models::NewUser) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users 
            (privy_id)
            VALUES ($1)
            RETURNING privy_id, created_at, updated_at
            "#,
        )
        .bind(&new_user.privy_id)
        .fetch_one(&self.db)
        .await
    }

    async fn get_user(&self, privy_id: PrivyId) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT privy_id, created_at, updated_at
            FROM users 
            WHERE privy_id = $1
            "#,
        )
        .bind(privy_id)
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
            SELECT privy_id, created_at, updated_at
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

    async fn delete_user(&self, privy_id: PrivyId) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE privy_id = $1")
            .bind(privy_id)
            .execute(&self.db)
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
