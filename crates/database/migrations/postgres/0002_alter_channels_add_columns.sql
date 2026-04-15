-- Migration 0002: Add api_version and pricing_region columns to channels

ALTER TABLE channels ADD COLUMN IF NOT EXISTS api_version VARCHAR(32) DEFAULT 'default';
ALTER TABLE channels ADD COLUMN IF NOT EXISTS pricing_region VARCHAR(32) DEFAULT 'international'
