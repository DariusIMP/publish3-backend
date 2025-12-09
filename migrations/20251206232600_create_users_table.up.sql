CREATE TABLE users (
    privy_id VARCHAR(255) NOT NULL PRIMARY KEY, -- Primary key instead of UUID id
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(100) NOT NULL UNIQUE,
    full_name VARCHAR(100),
    avatar_s3key VARCHAR DEFAULT NULL, -- S3 key for user's avatar image
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
    -- Removed: id, is_active, is_admin
);

CREATE INDEX idx_users_email ON users (email);

CREATE INDEX idx_users_username ON users (username);
