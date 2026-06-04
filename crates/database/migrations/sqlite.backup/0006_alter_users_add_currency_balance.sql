-- Migration 0006: Add preferred_currency and dual-currency wallet columns to users
-- balance_usd and balance_cny stored as BIGINT nanodollars (9 decimal precision, i64).
-- The runner treats "duplicate column" errors as a no-op.

ALTER TABLE users ADD COLUMN preferred_currency VARCHAR(10) DEFAULT 'USD';
ALTER TABLE users ADD COLUMN balance_usd BIGINT DEFAULT 0;
ALTER TABLE users ADD COLUMN balance_cny BIGINT DEFAULT 0
