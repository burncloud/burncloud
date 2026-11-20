-- 迁移 0020：持久化完整模型身份和摘要校验结果。
ALTER TABLE sys_downloads ADD COLUMN model TEXT;
ALTER TABLE sys_downloads ADD COLUMN source TEXT;
ALTER TABLE sys_downloads ADD COLUMN revision TEXT;
ALTER TABLE sys_downloads ADD COLUMN variant_id TEXT;
ALTER TABLE sys_downloads ADD COLUMN artifact_uri TEXT;
ALTER TABLE sys_downloads ADD COLUMN checksum TEXT;
ALTER TABLE sys_downloads ADD COLUMN actual_sha256 TEXT;
CREATE INDEX IF NOT EXISTS idx_sys_downloads_model_identity ON sys_downloads(model, source, revision, variant_id, checksum);
