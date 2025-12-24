use async_trait::async_trait;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

use crate::db::sql::{PrivyId, SqlClient, models::PublicationAuthor};

#[async_trait]
pub trait PublicationAuthorOperations {
    async fn add_author_to_publication(
        &self,
        publication_id: Uuid,
        author_id: &PrivyId,
        author_order: Option<i32>,
    ) -> Result<(), sqlx::Error>;

    async fn remove_author_from_publication(
        &self,
        publication_id: Uuid,
        author_id: &PrivyId,
    ) -> Result<PgQueryResult, sqlx::Error>;

    async fn update_author_order(
        &self,
        publication_id: Uuid,
        author_id: &PrivyId,
        author_order: i32,
    ) -> Result<PgQueryResult, sqlx::Error>;

    async fn get_publication_authors(
        &self,
        publication_id: Uuid,
    ) -> Result<Vec<PublicationAuthor>, sqlx::Error>;

    async fn get_author_publications(
        &self,
        author_id: &PrivyId,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<super::models::Publication>, sqlx::Error>;

    async fn publication_has_author(
        &self,
        publication_id: Uuid,
        author_id: &PrivyId,
    ) -> Result<bool, sqlx::Error>;

    async fn set_publication_authors(
        &self,
        publication_id: Uuid,
        author_ids: &[PrivyId],
    ) -> Result<(), sqlx::Error>;

    async fn count_authors_for_publication(&self, publication_id: Uuid)
    -> Result<i64, sqlx::Error>;

    async fn count_publications_for_author(&self, author_id: &PrivyId) -> Result<i64, sqlx::Error>;
}

#[async_trait]
impl PublicationAuthorOperations for SqlClient {
    async fn add_author_to_publication(
        &self,
        publication_id: Uuid,
        author_id: &PrivyId,
        author_order: Option<i32>,
    ) -> Result<(), sqlx::Error> {
        // Get the next order if not provided
        let order = match author_order {
            Some(order) => order,
            None => {
                let count: i32 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM publication_authors WHERE publication_id = $1",
                )
                .bind(publication_id)
                .fetch_one(&self.db)
                .await?;
                count + 1
            }
        };

        sqlx::query(
            r#"
            INSERT INTO publication_authors (publication_id, author_id, author_order)
            VALUES ($1, $2, $3)
            ON CONFLICT (publication_id, author_id) DO UPDATE
            SET author_order = EXCLUDED.author_order
            "#,
        )
        .bind(publication_id)
        .bind(author_id)
        .bind(order)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn remove_author_from_publication(
        &self,
        publication_id: Uuid,
        author_id: &PrivyId,
    ) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM publication_authors WHERE publication_id = $1 AND author_id = $2")
            .bind(publication_id)
            .bind(author_id)
            .execute(&self.db)
            .await
    }

    async fn update_author_order(
        &self,
        publication_id: Uuid,
        author_id: &PrivyId,
        author_order: i32,
    ) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE publication_authors 
            SET author_order = $1
            WHERE publication_id = $2 AND author_id = $3
            "#,
        )
        .bind(author_order)
        .bind(publication_id)
        .bind(author_id)
        .execute(&self.db)
        .await
    }

    async fn get_publication_authors(
        &self,
        publication_id: Uuid,
    ) -> Result<Vec<PublicationAuthor>, sqlx::Error> {
        sqlx::query_as::<_, PublicationAuthor>(
            r#"
            SELECT publication_id, author_id, author_order
            FROM publication_authors 
            WHERE publication_id = $1
            ORDER BY author_order ASC
            "#,
        )
        .bind(publication_id)
        .fetch_all(&self.db)
        .await
    }

    async fn get_author_publications(
        &self,
        author_id: &PrivyId,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<super::models::Publication>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        sqlx::query_as::<_, super::models::Publication>(
            r#"
            SELECT p.id, p.user_id, p.title, p.about, p.tags, p.s3key, p.price, p.citation_royalty_bps, p.status, p.transaction_hash, p.created_at, p.updated_at
            FROM publications p
            INNER JOIN publication_authors pa ON p.id = pa.publication_id
            WHERE pa.author_id = $1
            ORDER BY p.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(author_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }

    async fn publication_has_author(
        &self,
        publication_id: Uuid,
        author_id: &PrivyId,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM publication_authors WHERE publication_id = $1 AND author_id = $2)",
        )
        .bind(publication_id)
        .bind(author_id)
        .fetch_one(&self.db)
        .await
    }

    async fn set_publication_authors(
        &self,
        publication_id: Uuid,
        author_ids: &[PrivyId],
    ) -> Result<(), sqlx::Error> {
        // Start a transaction
        let mut tx = self.db.begin().await?;

        // Remove all existing authors for this publication
        sqlx::query("DELETE FROM publication_authors WHERE publication_id = $1")
            .bind(publication_id)
            .execute(&mut *tx)
            .await?;

        // Add new authors with order
        for (index, author_id) in author_ids.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO publication_authors (publication_id, author_id, author_order)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(publication_id)
            .bind(author_id)
            .bind((index + 1) as i32)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn count_authors_for_publication(
        &self,
        publication_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM publication_authors WHERE publication_id = $1")
            .bind(publication_id)
            .fetch_one(&self.db)
            .await
    }

    async fn count_publications_for_author(&self, author_id: &PrivyId) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM publication_authors WHERE author_id = $1")
            .bind(author_id)
            .fetch_one(&self.db)
            .await
    }
}
