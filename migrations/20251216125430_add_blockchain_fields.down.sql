ALTER TABLE publications 
DROP COLUMN IF EXISTS transaction_hash,
DROP COLUMN IF EXISTS status,
DROP COLUMN IF EXISTS price,
DROP COLUMN IF EXISTS citation_royalty_bps;

ALTER TABLE publications 
DROP CONSTRAINT IF EXISTS valid_status;
