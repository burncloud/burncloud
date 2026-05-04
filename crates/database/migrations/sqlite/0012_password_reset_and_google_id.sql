ALTER TABLE user_accounts ADD COLUMN google_id TEXT;

CREATE TABLE IF NOT EXISTS password_reset_tokens (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    FOREIGN KEY(user_id) REFERENCES user_accounts(id) ON DELETE CASCADE
);
