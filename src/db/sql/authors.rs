use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::postgres::PgQueryResult;

use crate::db::sql::{PrivyId, SqlClient, models::Author};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuthorWithPublications {
    pub id: uuid::Uuid,
    pub privy_id: PrivyId,
    pub name: String,
    pub email: Option<String>,
    pub affiliation: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub publications_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuthorWithPurchaseCount {
    pub id: uuid::Uuid,
    pub privy_id: PrivyId,
    pub name: String,
    pub email: Option<String>,
    pub affiliation: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub purchase_count: i64,
}

#[async_trait]
pub trait AuthorOperations {
    async fn create_author(
        &self,
        new_author: &super::models::NewAuthor,
    ) -> Result<Author, sqlx::Error>;

    async fn get_author(&self, id: &uuid::Uuid) -> Result<Author, sqlx::Error>;

    async fn get_author_by_email(&self, email: &str) -> Result<Author, sqlx::Error>;

    async fn list_authors(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Author>, sqlx::Error>;

    async fn list_authors_with_publications(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<AuthorWithPublications>, sqlx::Error>;

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
    ) -> Result<PgQueryResult, sqlx::Error>;

    async fn delete_author(&self, privy_id: &PrivyId) -> Result<PgQueryResult, sqlx::Error>;

    async fn author_email_exists(&self, email: &str) -> Result<bool, sqlx::Error>;

    async fn count_authors(&self) -> Result<i64, sqlx::Error>;

    async fn get_author_publication_count(&self, privy_id: &uuid::Uuid)
    -> Result<i64, sqlx::Error>;

    async fn get_author_purchase_count(&self, privy_id: &uuid::Uuid) -> Result<i64, sqlx::Error>;

    async fn get_author_revenue(&self, privy_id: &uuid::Uuid) -> Result<i64, sqlx::Error>;

    async fn list_top_authors_by_purchases(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<AuthorWithPurchaseCount>, sqlx::Error>;
}

#[async_trait]
impl AuthorOperations for SqlClient {
    async fn create_author(
        &self,
        new_author: &super::models::NewAuthor,
    ) -> Result<Author, sqlx::Error> {
        sqlx::query_as::<_, Author>(
            r#"
            INSERT INTO authors (id, privy_id, name, email, affiliation)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, privy_id, name, email, affiliation, created_at, updated_at
            "#,
        )
        .bind(new_author.id)
        .bind(&new_author.privy_id)
        .bind(&new_author.name)
        .bind(&new_author.email)
        .bind(&new_author.affiliation)
        .fetch_one(&self.db)
        .await
    }

    async fn get_author(&self, id: &uuid::Uuid) -> Result<Author, sqlx::Error> {
        sqlx::query_as::<_, Author>(
            r#"
            SELECT id, privy_id, name, email, affiliation, created_at, updated_at
            FROM authors 
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.db)
        .await
    }

    async fn get_author_by_email(&self, email: &str) -> Result<Author, sqlx::Error> {
        sqlx::query_as::<_, Author>(
            r#"
            SELECT id, privy_id, name, email, affiliation, created_at, updated_at
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
            SELECT id, privy_id, name, email, affiliation, created_at, updated_at
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

    async fn list_authors_with_publications(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<AuthorWithPublications>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        sqlx::query_as::<_, AuthorWithPublications>(
            r#"
            SELECT 
                a.id,
                a.privy_id, 
                a.name, 
                a.email, 
                a.affiliation, 
                a.created_at, 
                a.updated_at,
                COALESCE((
                    SELECT COUNT(*) 
                    FROM publication_authors pa 
                    WHERE pa.author_id = a.id
                ), 0) as publications_count
            FROM authors a
            ORDER BY publications_count DESC, a.name ASC
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
            SELECT id, privy_id, name, email, affiliation, created_at, updated_at
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
    ) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE authors SET
            name = COALESCE($1, name),
            email = COALESCE($2, email),
            affiliation = COALESCE($3, affiliation),
            updated_at = NOW()
            WHERE privy_id = $4
            "#,
        )
        .bind(name)
        .bind(email)
        .bind(affiliation)
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

    async fn count_authors(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM authors")
            .fetch_one(&self.db)
            .await
    }

    async fn get_author_publication_count(&self, id: &uuid::Uuid) -> Result<i64, sqlx::Error> {
        let author = self.get_author(id).await?;
        sqlx::query_scalar(
            r#"
            SELECT COUNT(*) 
            FROM publication_authors 
            WHERE author_id = $1
            "#,
        )
        .bind(author.id)
        .fetch_one(&self.db)
        .await
    }

    async fn get_author_purchase_count(&self, id: &uuid::Uuid) -> Result<i64, sqlx::Error> {
        let author = self.get_author(id).await?;
        sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT p.id)
            FROM purchases p
            INNER JOIN publications pub ON p.publication_id = pub.id
            INNER JOIN publication_authors pa ON pub.id = pa.publication_id
            WHERE pa.author_id = $1
            "#,
        )
        .bind(author.id)
        .fetch_one(&self.db)
        .await
    }

    async fn get_author_revenue(&self, id: &uuid::Uuid) -> Result<i64, sqlx::Error> {
        let author = self.get_author(id).await?;
        sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(pub.price)::BIGINT, 0)
            FROM purchases p
            INNER JOIN publications pub ON p.publication_id = pub.id
            INNER JOIN publication_authors pa ON pub.id = pa.publication_id
            WHERE pa.author_id = $1
            "#,
        )
        .bind(author.id)
        .fetch_one(&self.db)
        .await
    }

    async fn list_top_authors_by_purchases(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<AuthorWithPurchaseCount>, sqlx::Error> {
        let limit = limit.unwrap_or(3);
        sqlx::query_as::<_, AuthorWithPurchaseCount>(
            r#"
            SELECT 
                a.id,
                a.privy_id, 
                a.name, 
                a.email, 
                a.affiliation, 
                a.created_at, 
                a.updated_at,
                COALESCE(COUNT(DISTINCT p.id), 0) as purchase_count
            FROM authors a
            LEFT JOIN publication_authors pa ON a.id = pa.author_id
            LEFT JOIN publications pub ON pa.publication_id = pub.id
            LEFT JOIN purchases p ON pub.id = p.publication_id
            GROUP BY a.id, a.privy_id, a.name, a.email, a.affiliation, a.created_at, a.updated_at
            ORDER BY purchase_count DESC, a.name ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await
    }
}
