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
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS users (
                    id TEXT PRIMARY KEY,
                    username TEXT UNIQUE NOT NULL,
                    password_hash TEXT NOT NULL,
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
                    inviter_id TEXT,
                    deleted_at TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
                CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
            "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS users (
                    id TEXT PRIMARY KEY,
                    username VARCHAR(191) UNIQUE NOT NULL,
                    password_hash VARCHAR(255) NOT NULL,
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
                    inviter_id TEXT,
                    deleted_at TIMESTAMP
                );
                CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
                CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
            "#
            }
            _ => "",
        };

        // 2. Channels Table
        let channels_sql = match kind.as_str() {
            "sqlite" => {
                r#"
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
            "#
            }
            "postgres" => {
                r#"
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
            "#
            }
            _ => "",
        };

        // 3. Abilities Table (The routing core)
        // New API uses a composite primary key (group, model, channel_id)
        let abilities_sql = match kind.as_str() {
            "sqlite" => {
                r#"
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
            "#
            }
            "postgres" => {
                r#"
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
            "#
            }
            _ => "",
        };

        // 4. Tokens Table (App Tokens)
        let tokens_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS tokens (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id TEXT NOT NULL,
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
            "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS tokens (
                    id SERIAL PRIMARY KEY,
                    user_id TEXT NOT NULL,
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
            "#
            }
            _ => "",
        };

        // 5. Prices Table (Model Pricing)
        let prices_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS prices (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    model TEXT NOT NULL UNIQUE,
                    input_price REAL NOT NULL DEFAULT 0,
                    output_price REAL NOT NULL DEFAULT 0,
                    currency TEXT DEFAULT 'USD',
                    alias_for TEXT,
                    created_at INTEGER,
                    updated_at INTEGER
                );
                CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model);
            "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS prices (
                    id SERIAL PRIMARY KEY,
                    model VARCHAR(255) NOT NULL UNIQUE,
                    input_price DOUBLE PRECISION NOT NULL DEFAULT 0,
                    output_price DOUBLE PRECISION NOT NULL DEFAULT 0,
                    currency VARCHAR(10) DEFAULT 'USD',
                    alias_for VARCHAR(255),
                    created_at BIGINT,
                    updated_at BIGINT
                );
                CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model);
            "#
            }
            _ => "",
        };

        // Execute all
        if !users_sql.is_empty() {
            pool.execute(users_sql).await?;
        }
        if !channels_sql.is_empty() {
            pool.execute(channels_sql).await?;
        }
        if !abilities_sql.is_empty() {
            pool.execute(abilities_sql).await?;
        }
        if !tokens_sql.is_empty() {
            pool.execute(tokens_sql).await?;
        }
        if !prices_sql.is_empty() {
            pool.execute(prices_sql).await?;
        }

        // Init Root User if not exists
        // Username: root, Password: 123456 (Should be changed)
        // Note: Password hash logic is needed here, but for now we skip or put a placeholder.
        // Using a simple check
        let check_root_sql = "SELECT count(*) FROM users WHERE username = 'root'";
        let count: i64 = match kind.as_str() {
            "sqlite" => sqlx::query_scalar(check_root_sql)
                .fetch_one(pool)
                .await
                .unwrap_or(0),
            "postgres" => sqlx::query_scalar(check_root_sql)
                .fetch_one(pool)
                .await
                .unwrap_or(0),
            _ => 0,
        };

        if count == 0 {
            // ... (user insert)
            // ...
        }

        // Init Default Token
        let check_token_sql = "SELECT count(*) FROM tokens WHERE key = 'sk-burncloud-demo'";
        let t_count: i64 = match kind.as_str() {
            "sqlite" => sqlx::query_scalar(check_token_sql)
                .fetch_one(pool)
                .await
                .unwrap_or(0),
            "postgres" => sqlx::query_scalar(check_token_sql)
                .fetch_one(pool)
                .await
                .unwrap_or(0),
            _ => 0,
        };

        if t_count == 0 {
            // User 'demo-user' must exist (created by UserDatabase::init)
            // created_time, accessed_time use current timestamp
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            let insert_token_sql = match kind.as_str() {
                "sqlite" => "INSERT INTO tokens (user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time) VALUES ('demo-user', 'sk-burncloud-demo', 1, 'Demo Token', -1, 1, 0, ?, ?, -1)",
                "postgres" => "INSERT INTO tokens (user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time) VALUES ('demo-user', 'sk-burncloud-demo', 1, 'Demo Token', -1, TRUE, 0, $1, $2, -1)",
                _ => ""
            };
            if !insert_token_sql.is_empty() {
                sqlx::query(insert_token_sql)
                    .bind(now)
                    .bind(now)
                    .execute(pool)
                    .await?;
                println!("Initialized demo token: sk-burncloud-demo");
            }
        }

        // Init Default Prices
        let check_prices_sql = "SELECT count(*) FROM prices";
        let p_count: i64 = match kind.as_str() {
            "sqlite" => sqlx::query_scalar(check_prices_sql)
                .fetch_one(pool)
                .await
                .unwrap_or(0),
            "postgres" => sqlx::query_scalar(check_prices_sql)
                .fetch_one(pool)
                .await
                .unwrap_or(0),
            _ => 0,
        };

        if p_count == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            // Default pricing (prices per 1M tokens)
            // Format: (model, input_price, output_price, alias_for)
            let default_prices = [
                // OpenAI models
                ("gpt-4", 30.0, 60.0, None),
                ("gpt-4-turbo", 10.0, 30.0, Some("gpt-4")),
                ("gpt-4o", 2.5, 10.0, None),
                ("gpt-4o-mini", 0.15, 0.60, None),
                ("gpt-3.5-turbo", 0.50, 1.50, None),
                // Anthropic models
                ("claude-3-opus", 15.0, 75.0, None),
                ("claude-3-sonnet", 3.0, 15.0, None),
                ("claude-3-haiku", 0.25, 1.25, None),
                ("claude-3-5-sonnet", 3.0, 15.0, None),
                // Google models
                ("gemini-1.5-pro", 3.5, 10.5, None),
                ("gemini-1.5-flash", 0.075, 0.30, None),
                ("gemini-pro", 0.50, 1.50, None),
            ];

            for (model, input_price, output_price, alias_for) in default_prices {
                let insert_sql = match kind.as_str() {
                    "sqlite" => "INSERT INTO prices (model, input_price, output_price, currency, alias_for, created_at, updated_at) VALUES (?, ?, ?, 'USD', ?, ?, ?)",
                    "postgres" => "INSERT INTO prices (model, input_price, output_price, currency, alias_for, created_at, updated_at) VALUES ($1, $2, $3, 'USD', $4, $5, $6)",
                    _ => "",
                };
                if !insert_sql.is_empty() {
                    sqlx::query(insert_sql)
                        .bind(model)
                        .bind(input_price)
                        .bind(output_price)
                        .bind(alias_for)
                        .bind(now)
                        .bind(now)
                        .execute(pool)
                        .await?;
                }
            }
            println!(
                "Initialized default pricing for {} models",
                default_prices.len()
            );
        }

        Ok(())
    }
}
