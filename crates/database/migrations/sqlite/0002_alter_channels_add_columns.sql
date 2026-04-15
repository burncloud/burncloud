-- Migration 0002: Add api_version and pricing_region columns to channels
-- SQLite does not support IF NOT EXISTS on ALTER TABLE.
-- The runner treats "duplicate column" errors as a no-op so this file is safe
-- to run against both fresh and pre-existing databases.

ALTER TABLE channels ADD COLUMN api_version VARCHAR(32) DEFAULT 'default';
ALTER TABLE channels ADD COLUMN pricing_region VARCHAR(32) DEFAULT 'international'
