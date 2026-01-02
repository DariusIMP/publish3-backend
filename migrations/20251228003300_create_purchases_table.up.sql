CREATE TABLE purchases (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    publication_id UUID NOT NULL REFERENCES publications(id),
    status VARCHAR(50) DEFAULT 'PENDING',
    transaction_hash VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT valid_status CHECK (status IN ('PENDING', 'PAID', 'SETTLED', 'FAILED'))
);

CREATE INDEX idx_purchases_user_id ON purchases (user_id);
CREATE INDEX idx_purchases_publication_id ON purchases (publication_id);
CREATE INDEX idx_purchases_status ON purchases (status);
