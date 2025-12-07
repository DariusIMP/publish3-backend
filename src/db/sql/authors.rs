use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgQueryResult, FromRow};
use uuid::Uuid;

use crate::db::sql::{models::Author, SqlClient};

#[async_trait]
pub trait AuthorOperations {
    async fn create_author(&self, new_author: &super::models::NewAuthor) -> Result<Author, sqlx::Error>;
    
    async fn get_author(&self, author_id: Uuid) -> Result<Author, sqlx::Error>;
    
    async fn get_author_by_email(&self, email: &str) -> Result<Author, sqlx::Error>;
    
    async fn list_authors(
        &self, 
        page: Option<i64>, 
        limit: Option<i64>
    ) -> Result<Vec<Author>, sqlx::Error>;
    
    async fn search_authors_by_name(
        &self,
        name_query: &str,
        page: Option<i64>,
        limit: Option<i64>
    ) -> Result<Vec<Author>, sqlx::Error>;
    
    async fn update_author(
        &self,
        author_id: Uuid,
        name: Option<&str>,
        email: Option<&str>,
        affiliation: Option<&str>,
    ) -> Result<PgQueryResult, sqlx::Error>;
    
    async fn delete_author(&self, author_id: Uuid) -> Result<PgQueryResult, sqlx::Error>;
    
    async fn author_email_exists(&self, email: &str) -> Result<bool, sqlx::Error>;
    
    async fn count_authors(&self) -> Result<i64, sqlx::Error>;
}

#[async_trait]
impl AuthorOperations for SqlClient {
    async fn create_author(&self, new_author: &super::models::NewAuthor) -> Result<Author, sqlx::Error> {
        sqlx::query_as::<_, Author>(
            r#"
            INSERT INTO authors (name, email, affiliation)
            VALUES ($1, $2, $3)
            RETURNING id, name, email, affiliation, created_at, updated_at
            "#,
        )
        .bind(&new_author.name)
        .bind(&new_author.email)
        .bind(&new_author.affiliation)
        .fetch_one(&self.db)
        .await
    }
    
    async fn get_author(&self, author_id: Uuid) -> Result<Author, sqlx::Error> {
        sqlx::query_as::<_, Author>(
            r#"
            SELECT id, name, email, affiliation, created_at, updated_at
            FROM authors 
            WHERE id = $1
            "#,
        )
        .bind(author_id)
        .fetch_one(&self.db)
        .await
    }
    
    async fn get_author_by_email(&self, email: &str) -> Result<Author, sqlx::Error> {
        sqlx::query_as::<_, Author>(
            r#"
            SELECT id, name, email, affiliation, created_at, updated_at
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
        limit: Option<i64>
    ) -> Result<Vec<Author>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;
        
        sqlx::query_as::<_, Author>(
            r#"
            SELECT id, name, email, affiliation, created_at, updated_at
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
        limit: Option<i64>
    ) -> Result<Vec<Author>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;
        let search_pattern = format!("%{}%", name_query);
        
        sqlx::query_as::<_, Author>(
            r#"
            SELECT id, name, email, affiliation, created_at, updated_at
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
        author_id: Uuid,
        name: Option<&str>,
        email: Option<&str>,
        affiliation: Option<&str>,
    ) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE authors SET
            name = COALESCE($1, name),
            email = COALESCE($2, email),
            affiliation = COALESCE($3, affiliation),
            updated_at = NOW()
            WHERE id = $4
            "#,
        )
        .bind(name)
        .bind(email)
        .bind(affiliation)
        .bind(author_id)
        .execute(&self.db)
        .await
    }
    
    async fn delete_author(&self, author_id: Uuid) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            "DELETE FROM authors WHERE id = $1",
        )
        .bind(author_id)
        .execute(&self.db)
        .await
    }
    
    async fn author_email_exists(&self, email: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM authors WHERE email = $1)",
        )
        .bind(email)
        .fetch_one(&self.db)
        .await
    }
    
    async fn count_authors(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM authors",
        )
        .fetch_one(&self.db)
        .await
    }
}
