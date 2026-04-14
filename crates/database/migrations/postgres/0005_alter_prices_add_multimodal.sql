-- Migration 0005: Add multimodal pricing columns to prices table
-- All prices stored as BIGINT nanodollars (9 decimal precision)

ALTER TABLE prices ADD COLUMN IF NOT EXISTS audio_output_price BIGINT;
ALTER TABLE prices ADD COLUMN IF NOT EXISTS reasoning_price BIGINT;
ALTER TABLE prices ADD COLUMN IF NOT EXISTS embedding_price BIGINT;
ALTER TABLE prices ADD COLUMN IF NOT EXISTS image_price BIGINT;
ALTER TABLE prices ADD COLUMN IF NOT EXISTS video_price BIGINT;
ALTER TABLE prices ADD COLUMN IF NOT EXISTS music_price BIGINT
