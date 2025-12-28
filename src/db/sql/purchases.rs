use async_trait::async_trait;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

use crate::db::sql::{SqlClient, models::{Purchase, NewPurchase}};

#[async_trait]
pub trait PurchaseOperations {
    async fn create_purchase(&self, new_purchase: &NewPurchase) -> Result<Purchase, sqlx::Error>;
    async fn get_purchase(&self, purchase_id: Uuid) -> Result<Purchase, sqlx::Error>;
    async fn list_purchases_by_user(&self, user_id: &str, page: Option<i64>, limit: Option<i64>) -> Result<Vec<Purchase>, sqlx::Error>;
    async fn list_purchases_by_publication(&self, publication_id: Uuid, page: Option<i64>, limit: Option<i64>) -> Result<Vec<Purchase>, sqlx::Error>;
    async fn update_purchase_status(&self, purchase_id: Uuid, status: &str, transaction_hash: Option<&str>) -> Result<PgQueryResult, sqlx::Error>;
    async fn count_purchases_by_user(&self, user_id: &str) -> Result<i64, sqlx::Error>;
    async fn has_user_purchased_publication(&self, user_id: &str, publication_id: Uuid) -> Result<bool, sqlx::Error>;
}

#[async_trait]
impl PurchaseOperations for SqlClient {
    async fn create_purchase(&self, new_purchase: &NewPurchase) -> Result<Purchase, sqlx::Error> {
        sqlx::query_as::<_, Purchase>(
            r#"
            INSERT INTO purchases (user_id, publication_id, amount, currency, status, transaction_hash)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, publication_id, amount, currency, status, transaction_hash, created_at, updated_at
            "#,
        )
        .bind(&new_purchase.user_id)
        .bind(new_purchase.publication_id)
        .bind(new_purchase.amount)
        .bind(new_purchase.currency.as_deref().unwrap_or("MOVE"))
        .bind(new_purchase.status.as_deref().unwrap_or("PENDING"))
        .bind(&new_purchase.transaction_hash)
        .fetch_one(&self.db)
        .await
    }

    async fn get_purchase(&self, purchase_id: Uuid) -> Result<Purchase, sqlx::Error> {
        sqlx::query_as::<_, Purchase>(
            r#"
            SELECT id, user_id, publication_id, amount, currency, status, transaction_hash, created_at, updated_at
            FROM purchases
            WHERE id = $1
            "#,
        )
        .bind(purchase_id)
        .fetch_one(&self.db)
        .await
    }


    async fn list_purchases_by_user(&self, user_id: &str, page: Option<i64>, limit: Option<i64>) -> Result<Vec<Purchase>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        sqlx::query_as::<_, Purchase>(
            r#"
            SELECT id, user_id, publication_id, amount, currency, status, transaction_hash, created_at, updated_at
            FROM purchases
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

    async fn list_purchases_by_publication(&self, publication_id: Uuid, page: Option<i64>, limit: Option<i64>) -> Result<Vec<Purchase>, sqlx::Error> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        sqlx::query_as::<_, Purchase>(
            r#"
            SELECT id, user_id, publication_id, amount, currency, status, transaction_hash, created_at, updated_at
            FROM purchases
            WHERE publication_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(publication_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
    }

    async fn update_purchase_status(&self, purchase_id: Uuid, status: &str, transaction_hash: Option<&str>) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE purchases SET
            status = $1,
            transaction_hash = COALESCE($2, transaction_hash),
            updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(status)
        .bind(transaction_hash)
        .bind(purchase_id)
        .execute(&self.db)
        .await
    }

    async fn count_purchases_by_user(&self, user_id: &str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM purchases WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await
    }

    async fn has_user_purchased_publication(&self, user_id: &str, publication_id: Uuid) -> Result<bool, sqlx::Error> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM purchases
            WHERE user_id = $1 AND publication_id = $2 AND status IN ('PAID', 'SETTLED')
            "#,
        )
        .bind(user_id)
        .bind(publication_id)
        .fetch_one(&self.db)
        .await?;
        Ok(count > 0)
    }
}
