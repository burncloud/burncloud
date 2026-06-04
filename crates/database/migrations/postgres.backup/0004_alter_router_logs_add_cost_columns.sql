-- Migration 0004: Add per-type token counts and cost breakdown columns to router_logs
-- Cost columns use BIGINT (int8) to avoid int4 overflow at ~$2.10/request with nanodollars

ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS cache_write_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS audio_input_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS audio_output_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS image_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS embedding_tokens INTEGER DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS input_cost BIGINT DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS output_cost BIGINT DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS cache_read_cost BIGINT DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS cache_write_cost BIGINT DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS audio_cost BIGINT DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS image_cost BIGINT DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS video_cost BIGINT DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS reasoning_cost BIGINT DEFAULT 0;
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS embedding_cost BIGINT DEFAULT 0
