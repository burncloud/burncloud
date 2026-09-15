-- Migration 0008: Add extended pricing columns to prices table (v0.2.0+)
-- JSON columns store nanodollar pricing for voices, video, ASR, and realtime APIs.
-- The runner treats "duplicate column" errors as a no-op.

ALTER TABLE prices ADD COLUMN voices_pricing TEXT;
ALTER TABLE prices ADD COLUMN video_pricing TEXT;
ALTER TABLE prices ADD COLUMN asr_pricing TEXT;
ALTER TABLE prices ADD COLUMN realtime_pricing TEXT;
ALTER TABLE prices ADD COLUMN model_type TEXT
