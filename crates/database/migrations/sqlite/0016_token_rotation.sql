-- Migration 0016: Token rotation and security enhancements (SQLite)
-- Adds key versioning, old key hash for transition period, and IP whitelist

-- Add columns for token rotation
ALTER TABLE router_tokens ADD COLUMN key_version INTEGER DEFAULT 1;
ALTER TABLE router_tokens ADD COLUMN old_key_hash TEXT;
ALTER TABLE router_tokens ADD COLUMN old_key_expires_at INTEGER DEFAULT 0;
ALTER TABLE router_tokens ADD COLUMN ip_whitelist TEXT;
ALTER TABLE router_tokens ADD COLUMN key_prefix TEXT DEFAULT 'bc_live_';
ALTER TABLE router_tokens ADD COLUMN created_at INTEGER DEFAULT 0;
ALTER TABLE router_tokens ADD COLUMN last_rotated_at INTEGER DEFAULT 0;

-- Create index for faster lookups by user_id
CREATE INDEX IF NOT EXISTS idx_router_tokens_user_id ON router_tokens(user_id);
