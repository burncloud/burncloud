-- Migration 0008: Add extended pricing columns to prices table (v0.2.0+)
-- JSON columns store nanodollar pricing for voices, video, ASR, and realtime APIs

ALTER TABLE prices ADD COLUMN IF NOT EXISTS voices_pricing TEXT;
ALTER TABLE prices ADD COLUMN IF NOT EXISTS video_pricing TEXT;
ALTER TABLE prices ADD COLUMN IF NOT EXISTS asr_pricing TEXT;
ALTER TABLE prices ADD COLUMN IF NOT EXISTS realtime_pricing TEXT;
ALTER TABLE prices ADD COLUMN IF NOT EXISTS model_type VARCHAR(32)
