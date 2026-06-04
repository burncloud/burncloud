-- Migration 0016: Token rotation and security enhancements (PostgreSQL)
-- Adds key versioning, old key hash for transition period, and IP whitelist

-- Add columns for token rotation
ALTER TABLE router_tokens ADD COLUMN IF NOT EXISTS key_version INTEGER DEFAULT 1;
ALTER TABLE router_tokens ADD COLUMN IF NOT EXISTS old_key_hash TEXT;
ALTER TABLE router_tokens ADD COLUMN IF NOT EXISTS old_key_expires_at BIGINT DEFAULT 0;
ALTER TABLE router_tokens ADD COLUMN IF NOT EXISTS ip_whitelist TEXT;
ALTER TABLE router_tokens ADD COLUMN IF NOT EXISTS key_prefix VARCHAR(16) DEFAULT 'bc_live_';
ALTER TABLE router_tokens ADD COLUMN IF NOT EXISTS created_at BIGINT DEFAULT 0;
ALTER TABLE router_tokens ADD COLUMN IF NOT EXISTS last_rotated_at BIGINT DEFAULT 0;

-- Create index for faster lookups by user_id
CREATE INDEX IF NOT EXISTS idx_router_tokens_user_id ON router_tokens(user_id);
