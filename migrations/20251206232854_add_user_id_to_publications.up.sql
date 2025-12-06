ALTER TABLE publications 
ADD COLUMN user_id UUID REFERENCES users(id) ON DELETE SET NULL;

CREATE INDEX idx_publications_user_id ON publications(user_id);
