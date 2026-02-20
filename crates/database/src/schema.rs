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
                    remark TEXT,
                    api_version VARCHAR(32) DEFAULT 'default'
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
                    remark VARCHAR(255),
                    api_version VARCHAR(32) DEFAULT 'default'
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

        // 5. Prices Table (Model Pricing) with Advanced Pricing Fields
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
                    updated_at INTEGER,
                    cache_read_price REAL,
                    cache_creation_price REAL,
                    batch_input_price REAL,
                    batch_output_price REAL,
                    priority_input_price REAL,
                    priority_output_price REAL,
                    audio_input_price REAL,
                    full_pricing TEXT,
                    original_currency TEXT,
                    original_input_price REAL,
                    original_output_price REAL
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
                    updated_at BIGINT,
                    cache_read_price DOUBLE PRECISION,
                    cache_creation_price DOUBLE PRECISION,
                    batch_input_price DOUBLE PRECISION,
                    batch_output_price DOUBLE PRECISION,
                    priority_input_price DOUBLE PRECISION,
                    priority_output_price DOUBLE PRECISION,
                    audio_input_price DOUBLE PRECISION,
                    full_pricing TEXT,
                    original_currency VARCHAR(10),
                    original_input_price DOUBLE PRECISION,
                    original_output_price DOUBLE PRECISION
                );
                CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model);
            "#
            }
            _ => "",
        };

        // 6. Protocol Configs Table (Dynamic Protocol Adapter Configuration)
        let protocol_configs_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS protocol_configs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    channel_type INTEGER NOT NULL,
                    api_version VARCHAR(32) NOT NULL,
                    is_default BOOLEAN DEFAULT 0,
                    chat_endpoint VARCHAR(255),
                    embed_endpoint VARCHAR(255),
                    models_endpoint VARCHAR(255),
                    request_mapping TEXT,
                    response_mapping TEXT,
                    detection_rules TEXT,
                    created_at INTEGER,
                    updated_at INTEGER,
                    UNIQUE(channel_type, api_version)
                );
                CREATE INDEX IF NOT EXISTS idx_protocol_configs_type ON protocol_configs(channel_type);
                CREATE INDEX IF NOT EXISTS idx_protocol_configs_version ON protocol_configs(api_version);
            "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS protocol_configs (
                    id SERIAL PRIMARY KEY,
                    channel_type INTEGER NOT NULL,
                    api_version VARCHAR(32) NOT NULL,
                    is_default BOOLEAN DEFAULT FALSE,
                    chat_endpoint VARCHAR(255),
                    embed_endpoint VARCHAR(255),
                    models_endpoint VARCHAR(255),
                    request_mapping TEXT,
                    response_mapping TEXT,
                    detection_rules TEXT,
                    created_at BIGINT,
                    updated_at BIGINT,
                    UNIQUE(channel_type, api_version)
                );
                CREATE INDEX IF NOT EXISTS idx_protocol_configs_type ON protocol_configs(channel_type);
                CREATE INDEX IF NOT EXISTS idx_protocol_configs_version ON protocol_configs(api_version);
            "#
            }
            _ => "",
        };

        // 7. Model Capabilities Table (synced from LiteLLM)
        let model_capabilities_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS model_capabilities (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    model TEXT NOT NULL UNIQUE,
                    context_window INTEGER,
                    max_output_tokens INTEGER,
                    supports_vision BOOLEAN DEFAULT 0,
                    supports_function_calling BOOLEAN DEFAULT 0,
                    input_price REAL,
                    output_price REAL,
                    synced_at INTEGER
                );
                CREATE INDEX IF NOT EXISTS idx_model_capabilities_model ON model_capabilities(model);
            "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS model_capabilities (
                    id SERIAL PRIMARY KEY,
                    model VARCHAR(255) NOT NULL UNIQUE,
                    context_window BIGINT,
                    max_output_tokens BIGINT,
                    supports_vision BOOLEAN DEFAULT FALSE,
                    supports_function_calling BOOLEAN DEFAULT FALSE,
                    input_price DOUBLE PRECISION,
                    output_price DOUBLE PRECISION,
                    synced_at BIGINT
                );
                CREATE INDEX IF NOT EXISTS idx_model_capabilities_model ON model_capabilities(model);
            "#
            }
            _ => "",
        };

        // 8. Tiered Pricing Table (for models with tiered pricing like Qwen)
        let tiered_pricing_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS tiered_pricing (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    model TEXT NOT NULL,
                    region TEXT,
                    tier_start INTEGER NOT NULL,
                    tier_end INTEGER,
                    input_price REAL NOT NULL,
                    output_price REAL NOT NULL,
                    UNIQUE(model, region, tier_start)
                );
                CREATE INDEX IF NOT EXISTS idx_tiered_pricing_model ON tiered_pricing(model);
            "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS tiered_pricing (
                    id SERIAL PRIMARY KEY,
                    model VARCHAR(255) NOT NULL,
                    region VARCHAR(32),
                    tier_start BIGINT NOT NULL,
                    tier_end BIGINT,
                    input_price DOUBLE PRECISION NOT NULL,
                    output_price DOUBLE PRECISION NOT NULL,
                    UNIQUE(model, region, tier_start)
                );
                CREATE INDEX IF NOT EXISTS idx_tiered_pricing_model ON tiered_pricing(model);
            "#
            }
            _ => "",
        };

        // 9. Exchange Rates Table (for multi-currency support)
        let exchange_rates_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS exchange_rates (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    from_currency TEXT NOT NULL,
                    to_currency TEXT NOT NULL,
                    rate REAL NOT NULL,
                    updated_at INTEGER,
                    UNIQUE(from_currency, to_currency)
                );
                CREATE INDEX IF NOT EXISTS idx_exchange_rates_from ON exchange_rates(from_currency);
            "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS exchange_rates (
                    id SERIAL PRIMARY KEY,
                    from_currency VARCHAR(10) NOT NULL,
                    to_currency VARCHAR(10) NOT NULL,
                    rate DOUBLE PRECISION NOT NULL,
                    updated_at BIGINT,
                    UNIQUE(from_currency, to_currency)
                );
                CREATE INDEX IF NOT EXISTS idx_exchange_rates_from ON exchange_rates(from_currency);
            "#
            }
            _ => "",
        };

        // 10. Prices V2 Table (multi-currency pricing with advanced pricing fields)
        let prices_v2_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS prices_v2 (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    model TEXT NOT NULL,
                    currency TEXT NOT NULL DEFAULT 'USD',
                    input_price REAL NOT NULL DEFAULT 0,
                    output_price REAL NOT NULL DEFAULT 0,
                    cache_read_input_price REAL,
                    cache_creation_input_price REAL,
                    batch_input_price REAL,
                    batch_output_price REAL,
                    priority_input_price REAL,
                    priority_output_price REAL,
                    audio_input_price REAL,
                    source TEXT,
                    region TEXT,
                    context_window INTEGER,
                    max_output_tokens INTEGER,
                    supports_vision BOOLEAN DEFAULT 0,
                    supports_function_calling BOOLEAN DEFAULT 0,
                    synced_at INTEGER,
                    created_at INTEGER,
                    updated_at INTEGER,
                    UNIQUE(model, currency, region)
                );
                CREATE INDEX IF NOT EXISTS idx_prices_v2_model ON prices_v2(model);
                CREATE INDEX IF NOT EXISTS idx_prices_v2_model_currency ON prices_v2(model, currency);
            "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS prices_v2 (
                    id SERIAL PRIMARY KEY,
                    model VARCHAR(255) NOT NULL,
                    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
                    input_price DOUBLE PRECISION NOT NULL DEFAULT 0,
                    output_price DOUBLE PRECISION NOT NULL DEFAULT 0,
                    cache_read_input_price DOUBLE PRECISION,
                    cache_creation_input_price DOUBLE PRECISION,
                    batch_input_price DOUBLE PRECISION,
                    batch_output_price DOUBLE PRECISION,
                    priority_input_price DOUBLE PRECISION,
                    priority_output_price DOUBLE PRECISION,
                    audio_input_price DOUBLE PRECISION,
                    source VARCHAR(64),
                    region VARCHAR(32),
                    context_window BIGINT,
                    max_output_tokens BIGINT,
                    supports_vision BOOLEAN DEFAULT FALSE,
                    supports_function_calling BOOLEAN DEFAULT FALSE,
                    synced_at BIGINT,
                    created_at BIGINT,
                    updated_at BIGINT,
                    UNIQUE(model, currency, region)
                );
                CREATE INDEX IF NOT EXISTS idx_prices_v2_model ON prices_v2(model);
                CREATE INDEX IF NOT EXISTS idx_prices_v2_model_currency ON prices_v2(model, currency);
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
        if !protocol_configs_sql.is_empty() {
            pool.execute(protocol_configs_sql).await?;
        }
        if !model_capabilities_sql.is_empty() {
            pool.execute(model_capabilities_sql).await?;
        }
        if !tiered_pricing_sql.is_empty() {
            pool.execute(tiered_pricing_sql).await?;
        }
        if !exchange_rates_sql.is_empty() {
            pool.execute(exchange_rates_sql).await?;
        }
        if !prices_v2_sql.is_empty() {
            pool.execute(prices_v2_sql).await?;
        }

        // Migration: Add api_version column to channels if it doesn't exist
        // SQLite doesn't support IF NOT EXISTS for ALTER TABLE, so we try and ignore errors
        if kind == "sqlite" {
            let _ = sqlx::query(
                "ALTER TABLE channels ADD COLUMN api_version VARCHAR(32) DEFAULT 'default'",
            )
            .execute(pool)
            .await;
        } else if kind == "postgres" {
            let _ = sqlx::query("ALTER TABLE channels ADD COLUMN IF NOT EXISTS api_version VARCHAR(32) DEFAULT 'default'")
                .execute(pool)
                .await;
        }

        // Migration: Add pricing_region column to channels if it doesn't exist
        // Supports 'cn', 'international', and NULL (universal)
        if kind == "sqlite" {
            let _ = sqlx::query(
                "ALTER TABLE channels ADD COLUMN pricing_region VARCHAR(32) DEFAULT 'international'",
            )
            .execute(pool)
            .await;
        } else if kind == "postgres" {
            let _ = sqlx::query("ALTER TABLE channels ADD COLUMN IF NOT EXISTS pricing_region VARCHAR(32) DEFAULT 'international'")
                .execute(pool)
                .await;
        }

        // Migration: Add advanced pricing columns to prices table if they don't exist
        // Note: full_pricing is TEXT, others are REAL/DOUBLE PRECISION
        let advanced_pricing_columns_real = [
            "cache_read_price",
            "cache_creation_price",
            "batch_input_price",
            "batch_output_price",
            "priority_input_price",
            "priority_output_price",
            "audio_input_price",
        ];

        for column in advanced_pricing_columns_real {
            if kind == "sqlite" {
                let sql = format!("ALTER TABLE prices ADD COLUMN {} REAL", column);
                let _ = sqlx::query(&sql).execute(pool).await;
            } else if kind == "postgres" {
                let sql = format!(
                    "ALTER TABLE prices ADD COLUMN IF NOT EXISTS {} DOUBLE PRECISION",
                    column
                );
                let _ = sqlx::query(&sql).execute(pool).await;
            }
        }

        // Add full_pricing column as TEXT separately
        if kind == "sqlite" {
            let _ = sqlx::query("ALTER TABLE prices ADD COLUMN full_pricing TEXT")
                .execute(pool)
                .await;
        } else if kind == "postgres" {
            let _ = sqlx::query("ALTER TABLE prices ADD COLUMN IF NOT EXISTS full_pricing TEXT")
                .execute(pool)
                .await;
        }

        // Migration: Add multi-currency fields to prices table
        let multi_currency_columns = [
            ("original_currency", "VARCHAR(10)"),
            ("original_input_price", "REAL"),
            ("original_output_price", "REAL"),
        ];

        for (column, col_type) in multi_currency_columns {
            if kind == "sqlite" {
                let sql = format!("ALTER TABLE prices ADD COLUMN {} {}", column, col_type);
                let _ = sqlx::query(&sql).execute(pool).await;
            } else if kind == "postgres" {
                let pg_type = if col_type == "REAL" { "DOUBLE PRECISION" } else { col_type };
                let sql = format!("ALTER TABLE prices ADD COLUMN IF NOT EXISTS {} {}", column, pg_type);
                let _ = sqlx::query(&sql).execute(pool).await;
            }
        }

        // Migration: Add preferred_currency column to users table
        if kind == "sqlite" {
            let _ = sqlx::query(
                "ALTER TABLE users ADD COLUMN preferred_currency VARCHAR(10) DEFAULT 'USD'",
            )
            .execute(pool)
            .await;
        } else if kind == "postgres" {
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS preferred_currency VARCHAR(10) DEFAULT 'USD'")
                .execute(pool)
                .await;
        }

        // Migration: Add currency column to tiered_pricing table for multi-currency support
        if kind == "sqlite" {
            let _ = sqlx::query(
                "ALTER TABLE tiered_pricing ADD COLUMN currency VARCHAR(10) DEFAULT 'USD'",
            )
            .execute(pool)
            .await;
        } else if kind == "postgres" {
            let _ = sqlx::query("ALTER TABLE tiered_pricing ADD COLUMN IF NOT EXISTS currency VARCHAR(10) DEFAULT 'USD'")
                .execute(pool)
                .await;
        }

        // Migration: Migrate existing prices to prices_v2 table (USD only)
        // Only run if prices_v2 is empty but prices has data
        let prices_v2_count: i64 = match kind.as_str() {
            "sqlite" => sqlx::query_scalar("SELECT count(*) FROM prices_v2")
                .fetch_one(pool)
                .await
                .unwrap_or(0),
            "postgres" => sqlx::query_scalar("SELECT count(*) FROM prices_v2")
                .fetch_one(pool)
                .await
                .unwrap_or(0),
            _ => 0,
        };

        if prices_v2_count == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            // Insert from prices to prices_v2 (USD currency, no region)
            let migrate_sql = match kind.as_str() {
                "sqlite" => r#"
                    INSERT INTO prices_v2 (
                        model, currency, input_price, output_price,
                        cache_read_input_price, cache_creation_input_price,
                        batch_input_price, batch_output_price,
                        priority_input_price, priority_output_price,
                        audio_input_price, source, region,
                        created_at, updated_at
                    )
                    SELECT
                        model, 'USD', input_price, output_price,
                        cache_read_price, cache_creation_price,
                        batch_input_price, batch_output_price,
                        priority_input_price, priority_output_price,
                        audio_input_price, NULL, NULL,
                        ?, ?
                    FROM prices
                "#,
                "postgres" => r#"
                    INSERT INTO prices_v2 (
                        model, currency, input_price, output_price,
                        cache_read_input_price, cache_creation_input_price,
                        batch_input_price, batch_output_price,
                        priority_input_price, priority_output_price,
                        audio_input_price, source, region,
                        created_at, updated_at
                    )
                    SELECT
                        model, 'USD', input_price, output_price,
                        cache_read_price, cache_creation_price,
                        batch_input_price, batch_output_price,
                        priority_input_price, priority_output_price,
                        audio_input_price, NULL, NULL,
                        $1, $2
                    FROM prices
                    ON CONFLICT (model, currency, region) DO NOTHING
                "#,
                _ => "",
            };

            if !migrate_sql.is_empty() {
                let _ = sqlx::query(migrate_sql)
                    .bind(now)
                    .bind(now)
                    .execute(pool)
                    .await;
                println!("Migrated existing prices to prices_v2 table");
            }
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

        // Init Default Protocol Configs
        let check_protocol_sql = "SELECT count(*) FROM protocol_configs";
        let pc_count: i64 = match kind.as_str() {
            "sqlite" => sqlx::query_scalar(check_protocol_sql)
                .fetch_one(pool)
                .await
                .unwrap_or(0),
            "postgres" => sqlx::query_scalar(check_protocol_sql)
                .fetch_one(pool)
                .await
                .unwrap_or(0),
            _ => 0,
        };

        if pc_count == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            // Default protocol configs
            // channel_type values: 1=OpenAI, 2=Anthropic, 3=Azure, 4=Gemini, 5=Vertex
            let default_protocols: [(i32, &str, bool, Option<&str>, Option<&str>, Option<&str>);
                4] = [
                // OpenAI - default
                (
                    1,
                    "default",
                    true,
                    Some("/v1/chat/completions"),
                    Some("/v1/embeddings"),
                    Some("/v1/models"),
                ),
                // Anthropic - default
                (2, "2023-06-01", true, Some("/v1/messages"), None, None),
                // Azure OpenAI - default
                (
                    3,
                    "2024-02-01",
                    true,
                    Some("/deployments/{deployment_id}/chat/completions"),
                    Some("/deployments/{deployment_id}/embeddings"),
                    Some("/deployments?api-version=2024-02-01"),
                ),
                // Gemini - default
                (
                    4,
                    "v1",
                    true,
                    Some("/v1/models/{model}:generateContent"),
                    Some("/v1/models/{model}:embedContent"),
                    Some("/v1/models"),
                ),
            ];

            for (
                channel_type,
                api_version,
                is_default,
                chat_endpoint,
                embed_endpoint,
                models_endpoint,
            ) in default_protocols
            {
                let insert_sql = match kind.as_str() {
                    "sqlite" => "INSERT INTO protocol_configs (channel_type, api_version, is_default, chat_endpoint, embed_endpoint, models_endpoint, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    "postgres" => "INSERT INTO protocol_configs (channel_type, api_version, is_default, chat_endpoint, embed_endpoint, models_endpoint, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                    _ => "",
                };
                if !insert_sql.is_empty() {
                    sqlx::query(insert_sql)
                        .bind(channel_type)
                        .bind(api_version)
                        .bind(is_default)
                        .bind(chat_endpoint)
                        .bind(embed_endpoint)
                        .bind(models_endpoint)
                        .bind(now)
                        .bind(now)
                        .execute(pool)
                        .await?;
                }
            }
            println!(
                "Initialized default protocol configs for {} channel types",
                default_protocols.len()
            );
        }

        Ok(())
    }
}
