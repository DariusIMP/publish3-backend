-- Make publication fields NOT NULL
ALTER TABLE publications 
ALTER COLUMN user_id SET NOT NULL,
ALTER COLUMN about SET NOT NULL,
ALTER COLUMN s3key SET NOT NULL,
ALTER COLUMN price SET NOT NULL,
ALTER COLUMN citation_royalty_bps SET NOT NULL;

-- Remove default NULL values
ALTER TABLE publications 
ALTER COLUMN about DROP DEFAULT,
ALTER COLUMN s3key DROP DEFAULT;

-- Ensure price and citation_royalty_bps have appropriate defaults (they already have DEFAULT 0)
