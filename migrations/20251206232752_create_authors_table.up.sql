CREATE TABLE authors (
    privy_id VARCHAR(255) NOT NULL PRIMARY KEY REFERENCES users (privy_id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(100) UNIQUE,
    affiliation VARCHAR(200),
    wallet_id VARCHAR(255) NOT NULL REFERENCES wallets (wallet_id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT unique_wallet_id UNIQUE (wallet_id)
);

-- Create a junction table for publications and authors (many-to-many relationship)
CREATE TABLE publication_authors (
    publication_id UUID NOT NULL REFERENCES publications (id) ON DELETE CASCADE,
    author_id VARCHAR(255) NOT NULL REFERENCES authors (privy_id) ON DELETE CASCADE,
    author_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (publication_id, author_id)
);

CREATE INDEX idx_publication_authors_publication_id ON publication_authors (publication_id);
CREATE INDEX idx_publication_authors_author_id ON publication_authors (author_id);
CREATE INDEX idx_authors_wallet_id ON authors (wallet_id);
