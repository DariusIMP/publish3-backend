CREATE TABLE authors (
    id UUID NOT NULL PRIMARY KEY DEFAULT (uuid_generate_v4()),
    name VARCHAR(100) NOT NULL,
    email VARCHAR(100) UNIQUE,
    affiliation VARCHAR(200),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create a junction table for publications and authors (many-to-many relationship)
CREATE TABLE publication_authors (
    publication_id UUID NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES authors(id) ON DELETE CASCADE,
    author_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (publication_id, author_id)
);

CREATE INDEX idx_publication_authors_publication_id ON publication_authors(publication_id);
CREATE INDEX idx_publication_authors_author_id ON publication_authors(author_id);
