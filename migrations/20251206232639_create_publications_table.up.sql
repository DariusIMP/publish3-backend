CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE publications (
    id UUID NOT NULL PRIMARY KEY DEFAULT (uuid_generate_v4 ()),
    user_id VARCHAR(255) NOT NULL REFERENCES users (privy_id) ON DELETE SET NULL,
    title VARCHAR(512) NOT NULL,
    about TEXT NOT NULL,
    tags TEXT [] DEFAULT '{}',
    s3key VARCHAR NOT NULL, -- S3 key for the stored paper file
    transaction_hash VARCHAR(255),
    status VARCHAR(50) DEFAULT 'PENDING_ONCHAIN' NOT NULL,
    price BIGINT DEFAULT 0 NOT NULL,
    citation_royalty_bps BIGINT DEFAULT 0 NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT valid_status CHECK (
        status IN (
            'PENDING_ONCHAIN',
            'PUBLISHED',
            'FAILED'
        )
    )
);

CREATE INDEX idx_publications_transaction_hash ON publications (transaction_hash);
CREATE INDEX idx_publications_status ON publications (status);
