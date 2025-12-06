CREATE TABLE citations (
    id UUID NOT NULL PRIMARY KEY DEFAULT (uuid_generate_v4()),
    citing_publication_id UUID NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    cited_publication_id UUID NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    citation_context TEXT, -- Optional context/snippet where citation appears
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(citing_publication_id, cited_publication_id)
);

CREATE INDEX idx_citations_citing_publication_id ON citations(citing_publication_id);
CREATE INDEX idx_citations_cited_publication_id ON citations(cited_publication_id);
