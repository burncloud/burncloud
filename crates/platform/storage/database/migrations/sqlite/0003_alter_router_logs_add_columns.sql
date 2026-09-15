-- Migration 0003: Add model and multimodal token columns to router_logs
-- SQLite does not support IF NOT EXISTS on ALTER TABLE. The runner handles errors.

ALTER TABLE router_logs ADD COLUMN model TEXT;
ALTER TABLE router_logs ADD COLUMN cache_read_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN reasoning_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN pricing_region TEXT DEFAULT 'international';
ALTER TABLE router_logs ADD COLUMN video_tokens INTEGER DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_router_logs_model ON router_logs(model)
