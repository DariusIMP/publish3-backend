-- Remove wallet_address column from authors table
ALTER TABLE authors
DROP CONSTRAINT IF EXISTS unique_wallet_address;

DROP INDEX IF EXISTS idx_authors_wallet_address;

ALTER TABLE authors
DROP COLUMN wallet_address;
