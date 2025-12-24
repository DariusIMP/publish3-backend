CREATE TABLE user_wallets (
    user_id VARCHAR(255) NOT NULL REFERENCES users (privy_id) ON DELETE CASCADE,
    wallet_id VARCHAR(255) NOT NULL REFERENCES wallets (wallet_id) ON DELETE CASCADE,
    is_primary BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (user_id, wallet_id)
);

CREATE INDEX idx_user_wallets_user_id ON user_wallets (user_id);
CREATE INDEX idx_user_wallets_wallet_id ON user_wallets (wallet_id);
CREATE INDEX idx_user_wallets_is_primary ON user_wallets (is_primary) WHERE is_primary = TRUE;
