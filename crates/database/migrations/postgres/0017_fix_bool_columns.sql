-- Fix PostgreSQL bool columns: ensure proper BOOLEAN type
-- The sqlx Any driver needs consistent type handling.
-- These columns already use BOOLEAN in PostgreSQL, so this is a no-op migration
-- for PostgreSQL, but we keep the version number in sync with SQLite.

-- PostgreSQL already uses proper BOOLEAN type, so no changes needed.
-- This migration exists only to keep migration versions synchronized.
SELECT 1;
