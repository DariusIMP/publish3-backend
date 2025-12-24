use async_trait::async_trait;

use crate::db::sql::{SqlClient, models::{Wallet, UserWallet, NewWallet, NewUserWallet}};

#[async_trait]
pub trait WalletOperations {
    async fn create_wallet(&self, new_wallet: &NewWallet) -> Result<Wallet, sqlx::Error>;
    
    async fn create_user_wallet(&self, new_user_wallet: &NewUserWallet) -> Result<UserWallet, sqlx::Error>;
    
    async fn get_wallet(&self, wallet_id: &str) -> Result<Wallet, sqlx::Error>;
    
    async fn get_wallet_address(&self, wallet_id: &str) -> Result<String, sqlx::Error>;
    
    async fn get_primary_wallet(&self, user_id: &str) -> Result<Wallet, sqlx::Error>;
    
    async fn get_primary_wallets(&self, user_ids: &[String]) -> Result<Vec<Wallet>, sqlx::Error>;
    
    async fn wallet_exists(&self, wallet_id: &str) -> Result<bool, sqlx::Error>;
}

#[async_trait]
impl WalletOperations for SqlClient {
    async fn create_wallet(&self, new_wallet: &NewWallet) -> Result<Wallet, sqlx::Error> {
        sqlx::query_as::<_, Wallet>(
            r#"
            INSERT INTO wallets (wallet_id, wallet_address)
            VALUES ($1, $2)
            RETURNING wallet_id, wallet_address, created_at, updated_at
            "#,
        )
        .bind(&new_wallet.wallet_id)
        .bind(&new_wallet.wallet_address)
        .fetch_one(&self.db)
        .await
    }
    
    async fn create_user_wallet(&self, new_user_wallet: &NewUserWallet) -> Result<UserWallet, sqlx::Error> {
        sqlx::query_as::<_, UserWallet>(
            r#"
            INSERT INTO user_wallets (user_id, wallet_id, is_primary)
            VALUES ($1, $2, $3)
            RETURNING user_id, wallet_id, is_primary, created_at
            "#,
        )
        .bind(&new_user_wallet.user_id)
        .bind(&new_user_wallet.wallet_id)
        .bind(new_user_wallet.is_primary)
        .fetch_one(&self.db)
        .await
    }
    
    async fn get_wallet(&self, wallet_id: &str) -> Result<Wallet, sqlx::Error> {
        sqlx::query_as::<_, Wallet>(
            r#"
            SELECT wallet_id, wallet_address, created_at, updated_at
            FROM wallets 
            WHERE wallet_id = $1
            "#,
        )
        .bind(wallet_id)
        .fetch_one(&self.db)
        .await
    }
    
    async fn get_wallet_address(&self, wallet_id: &str) -> Result<String, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT wallet_address FROM wallets WHERE wallet_id = $1
            "#,
        )
        .bind(wallet_id)
        .fetch_one(&self.db)
        .await
    }
    
    async fn get_primary_wallet(&self, user_id: &str) -> Result<Wallet, sqlx::Error> {
        sqlx::query_as::<_, Wallet>(
            r#"
            SELECT w.wallet_id, w.wallet_address, w.created_at, w.updated_at
            FROM user_wallets uw
            JOIN wallets w ON uw.wallet_id = w.wallet_id
            WHERE uw.user_id = $1 AND uw.is_primary = TRUE
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await
    }
    
    async fn get_primary_wallets(&self, user_ids: &[String]) -> Result<Vec<Wallet>, sqlx::Error> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Create a list of placeholders for the IN clause
        let placeholders: Vec<String> = (1..=user_ids.len()).map(|i| format!("${}", i)).collect();
        let placeholders_str = placeholders.join(", ");

        let query_str = format!(
            r#"
            SELECT w.wallet_id, w.wallet_address, w.created_at, w.updated_at
            FROM user_wallets uw
            JOIN wallets w ON uw.wallet_id = w.wallet_id
            WHERE uw.user_id IN ({}) AND uw.is_primary = TRUE
            ORDER BY array_position(ARRAY[{}], uw.user_id)
            "#,
            placeholders_str, placeholders_str
        );

        let mut query = sqlx::query_as(&query_str);
        for user_id in user_ids {
            query = query.bind(user_id);
        }

        query.fetch_all(&self.db).await
    }
    
    async fn wallet_exists(&self, wallet_id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM wallets WHERE wallet_id = $1)")
            .bind(wallet_id)
            .fetch_one(&self.db)
            .await
    }
}
