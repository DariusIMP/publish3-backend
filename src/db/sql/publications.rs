use async_trait::async_trait;
use chrono::Utc;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

use crate::db::sql::{CitationOperations, SqlClient, models::Publication};

#[async_trait]
pub trait PublicationOperations {
    async fn create_publication(
        &self,
        new_publication: &super::models::NewPublication,
    ) -> Result<Publication, sqlx::Error>;

    async fn get_publication(&self, publication_id: Uuid) -> Result<Publication, sqlx::Error>;

    async fn list_publications(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Publication>, sqlx::Error>;

    async fn list_publications_with_authors(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<(Publication, Vec<super::models::Author>)>, sqlx::Error>;

    async fn list_publications_by_user(
        &self,
        user_id: &str,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Publication>, sqlx::Error>;

    async fn search_publications_by_title(
        &self,
        title_query: &str,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Publication>, sqlx::Error>;

    async fn search_publications_by_tag(
        &self,
        tag: &str,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Publication>, sqlx::Error>;

    async fn update_publication(
        &self,
        publication_id: Uuid,
        user_id: Option<&str>,
        title: Option<&str>,
        about: Option<&str>,
        tags: Option<&[String]>,
        s3key: Option<&str>,
    ) -> Result<PgQueryResult, sqlx::Error>;

    async fn delete_publication(&self, publication_id: Uuid) -> Result<PgQueryResult, sqlx::Error>;

    async fn count_publications(&self) -> Result<i64, sqlx::Error>;

    async fn count_publications_by_user(&self, user_id: &str) -> Result<i64, sqlx::Error>;

    async fn get_publication_authors(
        &self,
        publication_id: Uuid,
    ) -> Result<Vec<super::models::Author>, sqlx::Error>;

    async fn get_publication_authors_with_details(
        &self,
        publication_id: Uuid,
    ) -> Result<Vec<super::models::PublicationAuthorWithDetails>, sqlx::Error>;

    async fn get_publication_citations(
        &self,
        publication_id: Uuid,
    ) -> Result<Vec<super::models::Citation>, sqlx::Error>;

    async fn get_cited_by(&self, publication_id: Uuid) -> Result<Vec<Publication>, sqlx::Error>;

    async fn get_citation_count(&self, publication_id: Uuid) -> Result<i64, sqlx::Error>;
}

#[async_trait]
impl PublicationOperations for SqlClient {
    async fn create_publication(
        &self,
        new_publication: &super::models::NewPublication,
    ) -> Result<Publication, sqlx::Error> {
        sqlx::query_as::<_, Publication>(
            r#"
            INSERT INTO publications (user_id, title, about, tags, s3key)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, user_id, title, about, tags, s3key, created_at, updated_at
            "#,
        )
        .bind(&new_publication.user_id)
        .bind(&new_publication.title)
        .bind(&new_publication.about)
        .bind(new_publication.tags.as_deref().unwrap_or(&[]))
        .bind(&new_publication.s3key)
        .fetch_one(&self.db)
        .await
    }

    async fn get_publication(&self, publication_id: Uuid) -> Result<Publication, sqlx::Error> {
        sqlx::query_as::<_, Publication>(
            r#"
            SELECT id, user_id, title, about, tags, s3key, created_at, updated_at
            FROM publications 
            WHERE id = $1
            "#,
        )
        .bind(publication_id)
        .fetch_one(&self.db)
        .await
    }

    async fn list_publications(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Publication>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        sqlx::query_as::<_, Publication>(
            r#"
            SELECT id, user_id, title, about, tags, s3key, created_at, updated_at
            FROM publications 
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }

    async fn list_publications_with_authors(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<(Publication, Vec<super::models::Author>)>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        // First get publications
        let publications = sqlx::query_as::<_, Publication>(
            r#"
            SELECT id, user_id, title, about, tags, s3key, created_at, updated_at
            FROM publications 
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        // Then get authors for each publication using the new method
        let mut result = Vec::new();
        for publication in publications {
            let authors_with_details = self.get_publication_authors_with_details(publication.id).await?;
            
            // Convert PublicationAuthorWithDetails to Author format for frontend compatibility
            let authors: Vec<super::models::Author> = authors_with_details
                .into_iter()
                .map(|author_detail| super::models::Author {
                    privy_id: author_detail.author_id,
                    name: author_detail.author_name,
                    email: author_detail.author_email,
                    affiliation: author_detail.author_affiliation,
                    created_at: chrono::Utc::now(), // Placeholder - we don't have this info
                    updated_at: chrono::Utc::now(), // Placeholder - we don't have this info
                })
                .collect();
                
            result.push((publication, authors));
        }

        Ok(result)
    }

    async fn list_publications_by_user(
        &self,
        user_id: &str,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Publication>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        sqlx::query_as::<_, Publication>(
            r#"
            SELECT id, user_id, title, about, tags, s3key, created_at, updated_at
            FROM publications 
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }

    async fn search_publications_by_title(
        &self,
        title_query: &str,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Publication>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;
        let search_pattern = format!("%{}%", title_query);

        sqlx::query_as::<_, Publication>(
            r#"
            SELECT id, user_id, title, about, tags, s3key, created_at, updated_at
            FROM publications 
            WHERE title ILIKE $1
            ORDER BY title ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(search_pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }

    async fn search_publications_by_tag(
        &self,
        tag: &str,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Publication>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        sqlx::query_as::<_, Publication>(
            r#"
            SELECT id, user_id, title, about, tags, s3key, created_at, updated_at
            FROM publications 
            WHERE $1 = ANY(tags)
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tag)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }

    async fn update_publication(
        &self,
        publication_id: Uuid,
        user_id: Option<&str>,
        title: Option<&str>,
        about: Option<&str>,
        tags: Option<&[String]>,
        s3key: Option<&str>,
    ) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE publications SET
            user_id = COALESCE($1, user_id),
            title = COALESCE($2, title),
            about = COALESCE($3, about),
            tags = COALESCE($4, tags),
            s3key = COALESCE($5, s3key),
            updated_at = NOW()
            WHERE id = $6
            "#,
        )
        .bind(user_id)
        .bind(title)
        .bind(about)
        .bind(tags)
        .bind(s3key)
        .bind(publication_id)
        .execute(&self.db)
        .await
    }

    async fn delete_publication(&self, publication_id: Uuid) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM publications WHERE id = $1")
            .bind(publication_id)
            .execute(&self.db)
            .await
    }

    async fn count_publications(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM publications")
            .fetch_one(&self.db)
            .await
    }

    async fn count_publications_by_user(&self, user_id: &str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM publications WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&self.db)
            .await
    }

    async fn get_publication_authors(
        &self,
        publication_id: Uuid,
    ) -> Result<Vec<super::models::Author>, sqlx::Error> {
        sqlx::query_as::<_, super::models::Author>(
            r#"
            SELECT a.id, a.name, a.email, a.affiliation, a.created_at, a.updated_at
            FROM authors a
            INNER JOIN publication_authors pa ON a.id = pa.author_id
            WHERE pa.publication_id = $1
            ORDER BY pa.author_order ASC
            "#,
        )
        .bind(publication_id)
        .fetch_all(&self.db)
        .await
    }

    async fn get_publication_citations(
        &self,
        publication_id: Uuid,
    ) -> Result<Vec<super::models::Citation>, sqlx::Error> {
        sqlx::query_as::<_, super::models::Citation>(
            r#"
            SELECT id, citing_publication_id, cited_publication_id, created_at
            FROM citations 
            WHERE citing_publication_id = $1 OR cited_publication_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(publication_id)
        .fetch_all(&self.db)
        .await
    }

    async fn get_cited_by(&self, publication_id: Uuid) -> Result<Vec<Publication>, sqlx::Error> {
        sqlx::query_as::<_, Publication>(
            r#"
            SELECT p.id, p.user_id, p.title, p.about, p.tags, p.s3key, p.created_at, p.updated_at
            FROM publications p
            INNER JOIN citations c ON p.id = c.citing_publication_id
            WHERE c.cited_publication_id = $1
            ORDER BY c.created_at DESC
            "#,
        )
        .bind(publication_id)
        .fetch_all(&self.db)
        .await
    }

    async fn get_publication_authors_with_details(
        &self,
        publication_id: Uuid,
    ) -> Result<Vec<super::models::PublicationAuthorWithDetails>, sqlx::Error> {
        sqlx::query_as::<_, super::models::PublicationAuthorWithDetails>(
            r#"
            SELECT 
                pa.publication_id,
                pa.author_id,
                pa.author_order,
                a.name as author_name,
                a.email as author_email,
                a.affiliation as author_affiliation
            FROM publication_authors pa
            INNER JOIN authors a ON pa.author_id = a.privy_id
            WHERE pa.publication_id = $1
            ORDER BY pa.author_order ASC
            "#,
        )
        .bind(publication_id)
        .fetch_all(&self.db)
        .await
    }

    async fn get_citation_count(&self, publication_id: Uuid) -> Result<i64, sqlx::Error> {
        // Use the existing count_citations_to_publication method from CitationOperations
        self.count_citations_to_publication(publication_id).await
    }
}
