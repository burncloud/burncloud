-- Migration 0019: stable download identity and lifecycle metadata (PostgreSQL).
ALTER TABLE sys_downloads ADD COLUMN IF NOT EXISTS id BIGSERIAL;
ALTER TABLE sys_downloads ADD COLUMN IF NOT EXISTS url TEXT;
ALTER TABLE sys_downloads ADD COLUMN IF NOT EXISTS retry_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sys_downloads ADD COLUMN IF NOT EXISTS error TEXT;
ALTER TABLE sys_downloads ADD COLUMN IF NOT EXISTS final_path TEXT;
ALTER TABLE sys_downloads ADD COLUMN IF NOT EXISTS completed_at TIMESTAMP;

ALTER TABLE sys_downloads DROP CONSTRAINT IF EXISTS sys_downloads_pkey;
ALTER TABLE sys_downloads ADD CONSTRAINT sys_downloads_pkey PRIMARY KEY (id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_sys_downloads_gid ON sys_downloads(gid);
CREATE INDEX IF NOT EXISTS idx_sys_downloads_created_at ON sys_downloads(created_at);
