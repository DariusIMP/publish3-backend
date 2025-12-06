CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE publications (
    id UUID NOT NULL PRIMARY KEY DEFAULT (uuid_generate_v4()),
    title VARCHAR(100) NOT NULL,
    about TEXT DEFAULT NULL,
    tags TEXT[] DEFAULT '{}',
    s3key VARCHAR DEFAULT NULL, -- S3 key for the stored paper file
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
