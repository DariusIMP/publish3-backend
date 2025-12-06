DROP INDEX IF EXISTS idx_publications_user_id;
ALTER TABLE publications DROP COLUMN IF EXISTS user_id;
