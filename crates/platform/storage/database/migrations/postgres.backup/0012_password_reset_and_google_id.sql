ALTER TABLE user_accounts ADD COLUMN IF NOT EXISTS google_id TEXT;

CREATE TABLE IF NOT EXISTS password_reset_tokens (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    used_at TIMESTAMP,
    FOREIGN KEY(user_id) REFERENCES user_accounts(id) ON DELETE CASCADE
);
