use crate::{Database, Result};
use sqlx::Executor;

pub struct Schema;

impl Schema {
    pub async fn init(db: &Database) -> Result<()> {
        let pool = db.get_connection()?.pool();
        let kind = db.kind();

        // 1. Users Table
        // Note: 'group' is a reserved keyword in SQL, so we quote it or use a different name in DB if needed.
        // But New API uses 'group' in GORM. In raw SQL, we should quote it: "group" or `group`.
        let users_sql = match kind.as_str() {
            "sqlite" => r#"
                CREATE TABLE IF NOT EXISTS users (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    username TEXT UNIQUE NOT NULL,
                    password TEXT NOT NULL,
                    display_name TEXT DEFAULT '',
                    role INTEGER DEFAULT 1,
                    status INTEGER DEFAULT 1,
                    email TEXT,
                    github_id TEXT,
                    wechat_id TEXT,
                    access_token CHAR(32) UNIQUE,
                    quota INTEGER DEFAULT 0,
                    used_quota INTEGER DEFAULT 0,
                    request_count INTEGER DEFAULT 0,
                    `group` TEXT DEFAULT 'default',
                    aff_code VARCHAR(32) UNIQUE,
                    aff_count INTEGER DEFAULT 0,
                    aff_quota INTEGER DEFAULT 0,
                    inviter_id INTEGER,
                    deleted_at DATETIME
                );
                CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
                CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
            "#,
            "postgres" => r#"
                CREATE TABLE IF NOT EXISTS users (
                    id SERIAL PRIMARY KEY,
                    username VARCHAR(191) UNIQUE NOT NULL,
                    password VARCHAR(255) NOT NULL,
                    display_name VARCHAR(50) DEFAULT '',
                    role INTEGER DEFAULT 1,
                    status INTEGER DEFAULT 1,
                    email VARCHAR(50),
                    github_id VARCHAR(50),
                    wechat_id VARCHAR(50),
                    access_token CHAR(32) UNIQUE,
                    quota BIGINT DEFAULT 0,
                    used_quota BIGINT DEFAULT 0,
                    request_count INTEGER DEFAULT 0,
                    "group" VARCHAR(64) DEFAULT 'default',
                    aff_code VARCHAR(32) UNIQUE,
                    aff_count INTEGER DEFAULT 0,
                    aff_quota BIGINT DEFAULT 0,
                    inviter_id INTEGER,
                    deleted_at TIMESTAMP
                );
                CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
                CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
            "#,
            _ => "",
        };

        // 2. Channels Table
        let channels_sql = match kind.as_str() {
            "sqlite" => r#"
                CREATE TABLE IF NOT EXISTS channels (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    type INTEGER DEFAULT 0,
                    key TEXT NOT NULL,
                    status INTEGER DEFAULT 1,
                    name TEXT,
                    weight INTEGER DEFAULT 0,
                    created_time INTEGER,
                    test_time INTEGER,
                    response_time INTEGER,
                    base_url TEXT DEFAULT '',
                    models TEXT,
                    `group` TEXT DEFAULT 'default',
                    used_quota INTEGER DEFAULT 0,
                    model_mapping TEXT,
                    priority INTEGER DEFAULT 0,
                    auto_ban INTEGER DEFAULT 1,
                    other_info TEXT,
                    tag TEXT,
                    setting TEXT,
                    param_override TEXT,
                    header_override TEXT,
                    remark TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_channels_name ON channels(name);
                CREATE INDEX IF NOT EXISTS idx_channels_tag ON channels(tag);
            "#,
            "postgres" => r#"
                CREATE TABLE IF NOT EXISTS channels (
                    id SERIAL PRIMARY KEY,
                    type INTEGER DEFAULT 0,
                    key TEXT NOT NULL,
                    status INTEGER DEFAULT 1,
                    name VARCHAR(50),
                    weight INTEGER DEFAULT 0,
                    created_time BIGINT,
                    test_time BIGINT,
                    response_time INTEGER,
                    base_url VARCHAR(255) DEFAULT '',
                    models TEXT,
                    "group" VARCHAR(64) DEFAULT 'default',
                    used_quota BIGINT DEFAULT 0,
                    model_mapping TEXT,
                    priority BIGINT DEFAULT 0,
                    auto_ban INTEGER DEFAULT 1,
                    other_info TEXT,
                    tag VARCHAR(30),
                    setting TEXT,
                    param_override TEXT,
                    header_override TEXT,
                    remark VARCHAR(255)
                );
                CREATE INDEX IF NOT EXISTS idx_channels_name ON channels(name);
                CREATE INDEX IF NOT EXISTS idx_channels_tag ON channels(tag);
            "#,
            _ => "",
        };

        // 3. Abilities Table (The routing core)
        // New API uses a composite primary key (group, model, channel_id)
        let abilities_sql = match kind.as_str() {
            "sqlite" => r#"
                CREATE TABLE IF NOT EXISTS abilities (
                    `group` VARCHAR(64) NOT NULL,
                    model VARCHAR(255) NOT NULL,
                    channel_id INTEGER NOT NULL,
                    enabled BOOLEAN DEFAULT 1,
                    priority INTEGER DEFAULT 0,
                    weight INTEGER DEFAULT 0,
                    tag TEXT,
                    PRIMARY KEY (`group`, model, channel_id)
                );
                CREATE INDEX IF NOT EXISTS idx_abilities_model ON abilities(model);
                CREATE INDEX IF NOT EXISTS idx_abilities_channel_id ON abilities(channel_id);
            "#,
            "postgres" => r#"
                CREATE TABLE IF NOT EXISTS abilities (
                    "group" VARCHAR(64) NOT NULL,
                    model VARCHAR(255) NOT NULL,
                    channel_id INTEGER NOT NULL,
                    enabled BOOLEAN DEFAULT TRUE,
                    priority BIGINT DEFAULT 0,
                    weight INTEGER DEFAULT 0,
                    tag VARCHAR(30),
                    PRIMARY KEY ("group", model, channel_id)
                );
                CREATE INDEX IF NOT EXISTS idx_abilities_model ON abilities(model);
                CREATE INDEX IF NOT EXISTS idx_abilities_channel_id ON abilities(channel_id);
            "#,
            _ => "",
        };

        // 4. Tokens Table (App Tokens)
        let tokens_sql = match kind.as_str() {
            "sqlite" => r#"
                CREATE TABLE IF NOT EXISTS tokens (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id INTEGER NOT NULL,
                    key CHAR(48) NOT NULL,
                    status INTEGER DEFAULT 1,
                    name VARCHAR(255),
                    remain_quota INTEGER DEFAULT 0,
                    unlimited_quota BOOLEAN DEFAULT 0,
                    used_quota INTEGER DEFAULT 0,
                    created_time INTEGER,
                    accessed_time INTEGER,
                    expired_time INTEGER DEFAULT -1
                );
                CREATE INDEX IF NOT EXISTS idx_tokens_key ON tokens(key);
                CREATE INDEX IF NOT EXISTS idx_tokens_user_id ON tokens(user_id);
            "#,
            "postgres" => r#"
                CREATE TABLE IF NOT EXISTS tokens (
                    id SERIAL PRIMARY KEY,
                    user_id INTEGER NOT NULL,
                    key CHAR(48) NOT NULL,
                    status INTEGER DEFAULT 1,
                    name VARCHAR(255),
                    remain_quota BIGINT DEFAULT 0,
                    unlimited_quota BOOLEAN DEFAULT FALSE,
                    used_quota BIGINT DEFAULT 0,
                    created_time BIGINT,
                    accessed_time BIGINT,
                    expired_time BIGINT DEFAULT -1
                );
                CREATE INDEX IF NOT EXISTS idx_tokens_key ON tokens(key);
                CREATE INDEX IF NOT EXISTS idx_tokens_user_id ON tokens(user_id);
            "#,
            _ => "",
        };

        // Execute all
        if !users_sql.is_empty() { pool.execute(users_sql).await?; }
        if !channels_sql.is_empty() { pool.execute(channels_sql).await?; }
        if !abilities_sql.is_empty() { pool.execute(abilities_sql).await?; }
        if !tokens_sql.is_empty() { pool.execute(tokens_sql).await?; }

        // Init Root User if not exists
        // Username: root, Password: 123456 (Should be changed)
        // Note: Password hash logic is needed here, but for now we skip or put a placeholder.
        // Using a simple check
        let check_root_sql = "SELECT count(*) FROM users WHERE username = 'root'";
        let count: i64 = match kind.as_str() {
            "sqlite" => sqlx::query_scalar(check_root_sql).fetch_one(pool).await.unwrap_or(0),
            "postgres" => sqlx::query_scalar(check_root_sql).fetch_one(pool).await.unwrap_or(0),
            _ => 0
        };

        if count == 0 {
            // Insert default root user
            // IMPORTANT: Password should be hashed. We'll need a helper for this.
            // For now, we assume "123456" hash is pre-calculated or we leave it as plaintext (DANGEROUS - fix later).
            // Let's use a placeholder hash.
            let default_hash = "$2a$12$lX.q.q.q.q.q.q.q.q.q.q.q.q.q.q.q.q.q.q.q.q.q.q.q.q"; // Fake hash
            let insert_sql = match kind.as_str() {
                "sqlite" => "INSERT INTO users (username, password, role, status, quota, `group`) VALUES ('root', ?, 100, 1, 1000000, 'vip')",
                "postgres" => "INSERT INTO users (username, password, role, status, quota, \"group\") VALUES ('root', ?, 100, 1, 1000000, 'vip')",
                _ => ""
            };
            if !insert_sql.is_empty() {
                sqlx::query(insert_sql).bind(default_hash).execute(pool).await?;
                println!("Initialized root user.");
            }
        }

        Ok(())
    }
}
