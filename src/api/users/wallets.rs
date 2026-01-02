use privy_rs::generated::types::{CreateWalletBody, OwnerIdInput, WalletChainType};

use crate::{
    AppState, CONFIG,
    common::zresult::ZResult,
    db::sql::{NewUserWallet, NewWallet, PrivyId, UserOperations, WalletOperations},
    zerror,
};

pub(crate) async fn create_user_wallet(data: &AppState, user_privy_id: PrivyId) -> ZResult<()> {
    let user = data.sql_client.get_user_by_privy_id(user_privy_id.clone()).await
        .map_err(|err| {
            tracing::error!("Error finding user for privy_id {}: {}", user_privy_id, err);
            zerror!("User not found")
        })?;

    let privy = data.privy_client.clone();
    let wallet = privy
        .wallets()
        .create(
            Some(&user_privy_id),
            &CreateWalletBody {
                chain_type: WalletChainType::Movement,
                owner_id: Some(OwnerIdInput(CONFIG.privy_wallet_auth.clone())),
                owner: None,
                policy_ids: vec![],
                additional_signers: None,
            },
        )
        .await
        .map_err(|err| {
            tracing::error!("Failed to create user wallet: {}", err);
            zerror!("Failed to create wallet for user.")
        })?;

    let new_wallet = NewWallet {
        wallet_id: wallet.id.clone(),
        wallet_address: wallet.address.clone(),
    };

    data.sql_client
        .create_wallet(&new_wallet)
        .await
        .map_err(|err| {
            tracing::error!("Error creating wallet: {}", err);
            zerror!("Failed to create wallet")
        })?;

    let new_user_wallet = NewUserWallet {
        user_id: user.id,
        wallet_id: wallet.id.clone(),
        is_primary: true,
    };

    data.sql_client
        .create_user_wallet(&new_user_wallet)
        .await
        .map_err(|err| {
            tracing::error!("Error creating user_wallet association: {}", err);
            zerror!("Failed to associate wallet with user")
        })?;

    Ok(())
}
