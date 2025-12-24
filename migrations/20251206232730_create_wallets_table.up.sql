CREATE TABLE wallets (
    wallet_id VARCHAR(255) NOT NULL PRIMARY KEY, -- privy wallet id
    wallet_address VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_wallets_wallet_address ON wallets (wallet_address);
