-- Migration 0004: Add per-type token counts and cost breakdown columns to router_logs
-- SQLite INTEGER is 8-byte, safe for nanodollar cost values.
-- The runner treats "duplicate column" errors as a no-op.

ALTER TABLE router_logs ADD COLUMN cache_write_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN audio_input_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN audio_output_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN image_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN embedding_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN input_cost INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN output_cost INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN cache_read_cost INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN cache_write_cost INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN audio_cost INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN image_cost INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN video_cost INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN reasoning_cost INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN embedding_cost INTEGER DEFAULT 0
