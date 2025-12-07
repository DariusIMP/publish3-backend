use async_trait::async_trait;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

use crate::db::sql::{models::Citation, SqlClient};

#[async_trait]
pub trait CitationOperations {
    async fn create_citation(&self, new_citation: &super::models::NewCitation) -> Result<Citation, sqlx::Error>;
    
    async fn get_citation(&self, citation_id: Uuid) -> Result<Citation, sqlx::Error>;
    
    async fn get_citation_by_publications(
        &self,
        citing_publication_id: Uuid,
        cited_publication_id: Uuid,
    ) -> Result<Option<Citation>, sqlx::Error>;
    
    async fn list_citations(
        &self, 
        page: Option<i64>, 
        limit: Option<i64>
    ) -> Result<Vec<Citation>, sqlx::Error>;
    
    async fn list_citations_from_publication(
        &self,
        citing_publication_id: Uuid,
        page: Option<i64>,
        limit: Option<i64>
    ) -> Result<Vec<Citation>, sqlx::Error>;
    
    async fn list_citations_to_publication(
        &self,
        cited_publication_id: Uuid,
        page: Option<i64>,
        limit: Option<i64>
    ) -> Result<Vec<Citation>, sqlx::Error>;
    
    async fn update_citation(
        &self,
        citation_id: Uuid,
        citation_context: Option<&str>,
    ) -> Result<PgQueryResult, sqlx::Error>;
    
    async fn delete_citation(&self, citation_id: Uuid) -> Result<PgQueryResult, sqlx::Error>;
    
    async fn delete_citation_by_publications(
        &self,
        citing_publication_id: Uuid,
        cited_publication_id: Uuid,
    ) -> Result<PgQueryResult, sqlx::Error>;
    
    async fn count_citations(&self) -> Result<i64, sqlx::Error>;
    
    async fn count_citations_from_publication(&self, citing_publication_id: Uuid) -> Result<i64, sqlx::Error>;
    
    async fn count_citations_to_publication(&self, cited_publication_id: Uuid) -> Result<i64, sqlx::Error>;
}

#[async_trait]
impl CitationOperations for SqlClient {
    async fn create_citation(&self, new_citation: &super::models::NewCitation) -> Result<Citation, sqlx::Error> {
        sqlx::query_as::<_, Citation>(
            r#"
            INSERT INTO citations (citing_publication_id, cited_publication_id, citation_context)
            VALUES ($1, $2, $3)
            RETURNING id, citing_publication_id, cited_publication_id, citation_context, created_at
            "#,
        )
        .bind(new_citation.citing_publication_id)
        .bind(new_citation.cited_publication_id)
        .bind(&new_citation.citation_context)
        .fetch_one(&self.db)
        .await
    }
    
    async fn get_citation(&self, citation_id: Uuid) -> Result<Citation, sqlx::Error> {
        sqlx::query_as::<_, Citation>(
            r#"
            SELECT id, citing_publication_id, cited_publication_id, citation_context, created_at
            FROM citations 
            WHERE id = $1
            "#,
        )
        .bind(citation_id)
        .fetch_one(&self.db)
        .await
    }
    
    async fn get_citation_by_publications(
        &self,
        citing_publication_id: Uuid,
        cited_publication_id: Uuid,
    ) -> Result<Option<Citation>, sqlx::Error> {
        sqlx::query_as::<_, Citation>(
            r#"
            SELECT id, citing_publication_id, cited_publication_id, citation_context, created_at
            FROM citations 
            WHERE citing_publication_id = $1 AND cited_publication_id = $2
            "#,
        )
        .bind(citing_publication_id)
        .bind(cited_publication_id)
        .fetch_optional(&self.db)
        .await
    }
    
    async fn list_citations(
        &self, 
        page: Option<i64>, 
        limit: Option<i64>
    ) -> Result<Vec<Citation>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;
        
        sqlx::query_as::<_, Citation>(
            r#"
            SELECT id, citing_publication_id, cited_publication_id, citation_context, created_at
            FROM citations 
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }
    
    async fn list_citations_from_publication(
        &self,
        citing_publication_id: Uuid,
        page: Option<i64>,
        limit: Option<i64>
    ) -> Result<Vec<Citation>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;
        
        sqlx::query_as::<_, Citation>(
            r#"
            SELECT id, citing_publication_id, cited_publication_id, citation_context, created_at
            FROM citations 
            WHERE citing_publication_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(citing_publication_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }
    
    async fn list_citations_to_publication(
        &self,
        cited_publication_id: Uuid,
        page: Option<i64>,
        limit: Option<i64>
    ) -> Result<Vec<Citation>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;
        
        sqlx::query_as::<_, Citation>(
            r#"
            SELECT id, citing_publication_id, cited_publication_id, citation_context, created_at
            FROM citations 
            WHERE cited_publication_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(cited_publication_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }
    
    async fn update_citation(
        &self,
        citation_id: Uuid,
        citation_context: Option<&str>,
    ) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE citations SET
            citation_context = COALESCE($1, citation_context)
            WHERE id = $2
            "#,
        )
        .bind(citation_context)
        .bind(citation_id)
        .execute(&self.db)
        .await
    }
    
    async fn delete_citation(&self, citation_id: Uuid) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            "DELETE FROM citations WHERE id = $1",
        )
        .bind(citation_id)
        .execute(&self.db)
        .await
    }
    
    async fn delete_citation_by_publications(
        &self,
        citing_publication_id: Uuid,
        cited_publication_id: Uuid,
    ) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            "DELETE FROM citations WHERE citing_publication_id = $1 AND cited_publication_id = $2",
        )
        .bind(citing_publication_id)
        .bind(cited_publication_id)
        .execute(&self.db)
        .await
    }
    
    async fn count_citations(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM citations",
        )
        .fetch_one(&self.db)
        .await
    }
    
    async fn count_citations_from_publication(&self, citing_publication_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM citations WHERE citing_publication_id = $1",
        )
        .bind(citing_publication_id)
        .fetch_one(&self.db)
        .await
    }
    
    async fn count_citations_to_publication(&self, cited_publication_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM citations WHERE cited_publication_id = $1",
        )
        .bind(cited_publication_id)
        .fetch_one(&self.db)
        .await
    }
}
