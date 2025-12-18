ALTER TABLE publications
ADD COLUMN IF NOT EXISTS transaction_hash VARCHAR(255),
ADD COLUMN IF NOT EXISTS status VARCHAR(50) DEFAULT 'PENDING_ONCHAIN',
ADD COLUMN IF NOT EXISTS price BIGINT DEFAULT 0,
ADD COLUMN IF NOT EXISTS citation_royalty_bps BIGINT DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_publications_transaction_hash ON publications (transaction_hash);

CREATE INDEX IF NOT EXISTS idx_publications_status ON publications (status);

ALTER TABLE publications
ADD CONSTRAINT valid_status CHECK (
    status IN (
        'PENDING_ONCHAIN',
        'PUBLISHED',
        'FAILED'
    )
);
