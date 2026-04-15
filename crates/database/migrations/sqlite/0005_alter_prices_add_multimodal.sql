-- Migration 0005: Add multimodal pricing columns to prices table
-- All prices stored as BIGINT nanodollars (9 decimal precision).
-- The runner treats "duplicate column" errors as a no-op.

ALTER TABLE prices ADD COLUMN audio_output_price BIGINT;
ALTER TABLE prices ADD COLUMN reasoning_price BIGINT;
ALTER TABLE prices ADD COLUMN embedding_price BIGINT;
ALTER TABLE prices ADD COLUMN image_price BIGINT;
ALTER TABLE prices ADD COLUMN video_price BIGINT;
ALTER TABLE prices ADD COLUMN music_price BIGINT
