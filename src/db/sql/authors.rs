use async_trait::async_trait;
use sqlx::postgres::PgQueryResult;

use crate::db::sql::{PrivyId, SqlClient, models::Author};

#[async_trait]
pub trait AuthorOperations {
    async fn create_author(
        &self,
        new_author: &super::models::NewAuthor,
    ) -> Result<Author, sqlx::Error>;

    async fn get_author(&self, privy_id: &PrivyId) -> Result<Author, sqlx::Error>;

    async fn get_author_by_email(&self, email: &str) -> Result<Author, sqlx::Error>;

    async fn list_authors(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Author>, sqlx::Error>;

    async fn search_authors_by_name(
        &self,
        name_query: &str,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Author>, sqlx::Error>;

    async fn update_author(
        &self,
        privy_id: &PrivyId,
        name: Option<&str>,
        email: Option<&str>,
        affiliation: Option<&str>,
        wallet_id: Option<&str>,
    ) -> Result<PgQueryResult, sqlx::Error>;

    async fn delete_author(&self, privy_id: &PrivyId) -> Result<PgQueryResult, sqlx::Error>;

    async fn author_email_exists(&self, email: &str) -> Result<bool, sqlx::Error>;

    async fn author_wallet_id_exists(&self, wallet_id: &str)
    -> Result<bool, sqlx::Error>;

    async fn get_author_by_wallet_id(
        &self,
        wallet_id: &str,
    ) -> Result<Author, sqlx::Error>;

    async fn get_wallet_id(&self, privy_id: &PrivyId) -> Result<String, sqlx::Error>;

    async fn get_wallet_ids_by_privy_ids(
        &self,
        privy_ids: &[PrivyId],
    ) -> Result<Vec<String>, sqlx::Error>;

    async fn count_authors(&self) -> Result<i64, sqlx::Error>;
}

#[async_trait]
impl AuthorOperations for SqlClient {
    async fn create_author(
        &self,
        new_author: &super::models::NewAuthor,
    ) -> Result<Author, sqlx::Error> {
        sqlx::query_as::<_, Author>(
            r#"
            INSERT INTO authors (privy_id, name, email, affiliation, wallet_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING privy_id, name, email, affiliation, wallet_id, created_at, updated_at
            "#,
        )
        .bind(&new_author.privy_id)
        .bind(&new_author.name)
        .bind(&new_author.email)
        .bind(&new_author.affiliation)
        .bind(&new_author.wallet_id)
        .fetch_one(&self.db)
        .await
    }

    async fn get_author(&self, privy_id: &PrivyId) -> Result<Author, sqlx::Error> {
        sqlx::query_as::<_, Author>(
            r#"
            SELECT privy_id, name, email, affiliation, wallet_id, created_at, updated_at
            FROM authors 
            WHERE privy_id = $1
            "#,
        )
        .bind(privy_id)
        .fetch_one(&self.db)
        .await
    }

    async fn get_author_by_email(&self, email: &str) -> Result<Author, sqlx::Error> {
        sqlx::query_as::<_, Author>(
            r#"
            SELECT privy_id, name, email, affiliation, wallet_id, created_at, updated_at
            FROM authors 
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_one(&self.db)
        .await
    }

    async fn list_authors(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Author>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        sqlx::query_as::<_, Author>(
            r#"
            SELECT privy_id, name, email, affiliation, wallet_id, created_at, updated_at
            FROM authors 
            ORDER BY name ASC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }

    async fn search_authors_by_name(
        &self,
        name_query: &str,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Author>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;
        let search_pattern = format!("%{}%", name_query);

        sqlx::query_as::<_, Author>(
            r#"
            SELECT privy_id, name, email, affiliation, wallet_id, created_at, updated_at
            FROM authors 
            WHERE name ILIKE $1
            ORDER BY name ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(search_pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }

    async fn update_author(
        &self,
        privy_id: &PrivyId,
        name: Option<&str>,
        email: Option<&str>,
        affiliation: Option<&str>,
        wallet_id: Option<&str>,
    ) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE authors SET
            name = COALESCE($1, name),
            email = COALESCE($2, email),
            affiliation = COALESCE($3, affiliation),
            wallet_id = COALESCE($4, wallet_id),
            updated_at = NOW()
            WHERE privy_id = $5
            "#,
        )
        .bind(name)
        .bind(email)
        .bind(affiliation)
        .bind(wallet_id)
        .bind(privy_id)
        .execute(&self.db)
        .await
    }

    async fn delete_author(&self, privy_id: &PrivyId) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM authors WHERE privy_id = $1")
            .bind(privy_id)
            .execute(&self.db)
            .await
    }

    async fn author_email_exists(&self, email: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM authors WHERE email = $1)")
            .bind(email)
            .fetch_one(&self.db)
            .await
    }

    async fn author_wallet_id_exists(
        &self,
        wallet_id: &str,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM authors WHERE wallet_id = $1)")
            .bind(wallet_id)
            .fetch_one(&self.db)
            .await
    }

    async fn get_author_by_wallet_id(
        &self,
        wallet_id: &str,
    ) -> Result<Author, sqlx::Error> {
        sqlx::query_as::<_, Author>(
            r#"
            SELECT privy_id, name, email, affiliation, wallet_id, created_at, updated_at
            FROM authors 
            WHERE wallet_id = $1
            "#,
        )
        .bind(wallet_id)
        .fetch_one(&self.db)
        .await
    }

    async fn get_wallet_id(&self, privy_id: &PrivyId) -> Result<String, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT wallet_id FROM authors WHERE privy_id = $1
            "#,
        )
        .bind(privy_id)
        .fetch_one(&self.db)
        .await
    }

    async fn get_wallet_ids_by_privy_ids(
        &self,
        privy_ids: &[PrivyId],
    ) -> Result<Vec<String>, sqlx::Error> {
        if privy_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Create a list of placeholders for the IN clause
        let placeholders: Vec<String> = (1..=privy_ids.len()).map(|i| format!("${}", i)).collect();
        let placeholders_str = placeholders.join(", ");

        let query_str = format!(
            r#"
            SELECT wallet_id
            FROM authors 
            WHERE privy_id IN ({})
            ORDER BY array_position(ARRAY[{}], privy_id)
            "#,
            placeholders_str, placeholders_str
        );

        let mut query = sqlx::query_scalar(&query_str);
        for privy_id in privy_ids {
            query = query.bind(privy_id);
        }

        query.fetch_all(&self.db).await
    }

    async fn count_authors(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM authors")
            .fetch_one(&self.db)
            .await
    }
}
