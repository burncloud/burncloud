-- Migration 0003: Add model and multimodal token columns to router_logs

ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS model TEXT;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS cache_read_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS reasoning_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS pricing_region TEXT DEFAULT 'international';
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS video_tokens INTEGER DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_router_logs_model ON router_logs(model)
