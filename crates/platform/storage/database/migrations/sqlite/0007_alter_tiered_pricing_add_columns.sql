-- Migration 0007: Add currency and tier_type columns to tiered_pricing
-- The runner treats "duplicate column" errors as a no-op.

ALTER TABLE tiered_pricing ADD COLUMN currency VARCHAR(10) DEFAULT 'USD';
ALTER TABLE tiered_pricing ADD COLUMN tier_type VARCHAR(32) DEFAULT 'context_length'
