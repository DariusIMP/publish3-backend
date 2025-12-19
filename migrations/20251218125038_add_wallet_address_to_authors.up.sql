-- Add wallet_address column to authors table
ALTER TABLE authors
ADD COLUMN wallet_address VARCHAR(255) NOT NULL;

-- Add unique constraint to ensure each wallet address is unique per author
ALTER TABLE authors
ADD CONSTRAINT unique_wallet_address UNIQUE (wallet_address);

-- Create index for faster wallet address lookups
CREATE INDEX idx_authors_wallet_address ON authors (wallet_address);
