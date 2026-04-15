-- Migration 0007: Add currency and tier_type columns to tiered_pricing

ALTER TABLE tiered_pricing ADD COLUMN IF NOT EXISTS currency VARCHAR(10) DEFAULT 'USD';
ALTER TABLE tiered_pricing ADD COLUMN IF NOT EXISTS tier_type VARCHAR(32) DEFAULT 'context_length'
