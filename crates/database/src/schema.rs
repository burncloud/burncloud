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
        // Note: balance_usd and balance_cny are stored as BIGINT nanodollars (9 decimal precision)
        // Using i64 (signed BIGINT) for PostgreSQL compatibility, values must be non-negative
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
                    deleted_at TEXT,
                    balance_usd BIGINT DEFAULT 0,
                    balance_cny BIGINT DEFAULT 0,
                    preferred_currency VARCHAR(10) DEFAULT 'USD'
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
                    deleted_at TIMESTAMP,
                    balance_usd BIGINT DEFAULT 0,
                    balance_cny BIGINT DEFAULT 0,
                    preferred_currency VARCHAR(10) DEFAULT 'USD'
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
        // Note: Token-level quota fields (remain_quota, unlimited_quota, used_quota) are deprecated.
        // Token quotas are now managed at the user level via dual-currency wallet (balance_usd, balance_cny).
        // These fields are retained for backward compatibility and will be removed in a future migration.
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
        // ============================================================================
        // DEPRECATED: Use prices_v2 table instead.
        // - prices table uses REAL for prices (floating point precision issues)
        // - prices_v2 uses BIGINT for nanodollar prices (9 decimal precision)
        // - prices table has no region support
        // - prices_v2 supports regional pricing with UNIQUE(model, region) constraint
        // Migration path: All new pricing should use PriceV2Model, not PriceModel.
        // ============================================================================
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
        // Note: Prices are stored as BIGINT nanodollars (9 decimal precision)
        let tiered_pricing_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS tiered_pricing (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    model TEXT NOT NULL,
                    region TEXT,
                    tier_start INTEGER NOT NULL,
                    tier_end INTEGER,
                    input_price BIGINT NOT NULL,
                    output_price BIGINT NOT NULL,
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
                    input_price BIGINT NOT NULL,
                    output_price BIGINT NOT NULL,
                    UNIQUE(model, region, tier_start)
                );
                CREATE INDEX IF NOT EXISTS idx_tiered_pricing_model ON tiered_pricing(model);
            "#
            }
            _ => "",
        };

        // 9. Exchange Rates Table (for multi-currency support)
        // Note: Rate is stored as BIGINT scaled by 10^9 (9 decimal precision)
        let exchange_rates_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS exchange_rates (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    from_currency TEXT NOT NULL,
                    to_currency TEXT NOT NULL,
                    rate BIGINT NOT NULL,
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
                    rate BIGINT NOT NULL,
                    updated_at BIGINT,
                    UNIQUE(from_currency, to_currency)
                );
                CREATE INDEX IF NOT EXISTS idx_exchange_rates_from ON exchange_rates(from_currency);
            "#
            }
            _ => "",
        };

        // 10. Prices V2 Table (multi-currency pricing with advanced pricing fields)
        // Note: All prices are stored as BIGINT nanodollars (9 decimal precision)
        let prices_v2_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS prices_v2 (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    model TEXT NOT NULL,
                    currency TEXT NOT NULL DEFAULT 'USD',
                    input_price BIGINT NOT NULL DEFAULT 0,
                    output_price BIGINT NOT NULL DEFAULT 0,
                    cache_read_input_price BIGINT,
                    cache_creation_input_price BIGINT,
                    batch_input_price BIGINT,
                    batch_output_price BIGINT,
                    priority_input_price BIGINT,
                    priority_output_price BIGINT,
                    audio_input_price BIGINT,
                    source TEXT,
                    region TEXT,
                    context_window INTEGER,
                    max_output_tokens INTEGER,
                    supports_vision INTEGER DEFAULT 0,
                    supports_function_calling INTEGER DEFAULT 0,
                    synced_at INTEGER,
                    created_at INTEGER,
                    updated_at INTEGER,
                    UNIQUE(model, region)
                );
                CREATE INDEX IF NOT EXISTS idx_prices_v2_model ON prices_v2(model);
                CREATE INDEX IF NOT EXISTS idx_prices_v2_model_region ON prices_v2(model, region);
            "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS prices_v2 (
                    id SERIAL PRIMARY KEY,
                    model VARCHAR(255) NOT NULL,
                    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
                    input_price BIGINT NOT NULL DEFAULT 0,
                    output_price BIGINT NOT NULL DEFAULT 0,
                    cache_read_input_price BIGINT,
                    cache_creation_input_price BIGINT,
                    batch_input_price BIGINT,
                    batch_output_price BIGINT,
                    priority_input_price BIGINT,
                    priority_output_price BIGINT,
                    audio_input_price BIGINT,
                    source VARCHAR(64),
                    region VARCHAR(32),
                    context_window BIGINT,
                    max_output_tokens BIGINT,
                    supports_vision BOOLEAN DEFAULT FALSE,
                    supports_function_calling BOOLEAN DEFAULT FALSE,
                    synced_at BIGINT,
                    created_at BIGINT,
                    updated_at BIGINT,
                    UNIQUE(model, region)
                );
                CREATE INDEX IF NOT EXISTS idx_prices_v2_model ON prices_v2(model);
                CREATE INDEX IF NOT EXISTS idx_prices_v2_model_region ON prices_v2(model, region);
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

        // Migration: Add dual-currency wallet columns to users table
        // balance_usd and balance_cny are stored as BIGINT nanodollars (9 decimal precision)
        // Note: Using i64 (signed BIGINT) for PostgreSQL compatibility, values must be non-negative
        if kind == "sqlite" {
            let _ = sqlx::query(
                "ALTER TABLE users ADD COLUMN balance_usd BIGINT DEFAULT 0",
            )
            .execute(pool)
            .await;
            let _ = sqlx::query(
                "ALTER TABLE users ADD COLUMN balance_cny BIGINT DEFAULT 0",
            )
            .execute(pool)
            .await;
        } else if kind == "postgres" {
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS balance_usd BIGINT DEFAULT 0")
                .execute(pool)
                .await;
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS balance_cny BIGINT DEFAULT 0")
                .execute(pool)
                .await;
        }

        // Migration: Migrate existing quota to balance_usd for users with zero balance_usd
        // Quota system: 500000 quota = $1
        // Nanodollar conversion: $1 = 1_000_000_000 nanodollars
        // So: balance_usd = quota * 1_000_000_000 / 500000 = quota * 2000
        // Note: This only migrates users who haven't been migrated yet (balance_usd = 0)
        // Same SQL for both SQLite and PostgreSQL
        let _ = sqlx::query(
            "UPDATE users SET balance_usd = (quota - used_quota) * 2000 WHERE balance_usd = 0 AND quota > used_quota",
        )
        .execute(pool)
        .await;

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
            // Note: Convert REAL dollars to BIGINT nanodollars (multiply by 10^9)
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
                        model, 'USD',
                        CAST(ROUND(input_price * 1000000000) AS BIGINT),
                        CAST(ROUND(output_price * 1000000000) AS BIGINT),
                        CASE WHEN cache_read_price IS NOT NULL THEN CAST(ROUND(cache_read_price * 1000000000) AS BIGINT) END,
                        CASE WHEN cache_creation_price IS NOT NULL THEN CAST(ROUND(cache_creation_price * 1000000000) AS BIGINT) END,
                        CASE WHEN batch_input_price IS NOT NULL THEN CAST(ROUND(batch_input_price * 1000000000) AS BIGINT) END,
                        CASE WHEN batch_output_price IS NOT NULL THEN CAST(ROUND(batch_output_price * 1000000000) AS BIGINT) END,
                        CASE WHEN priority_input_price IS NOT NULL THEN CAST(ROUND(priority_input_price * 1000000000) AS BIGINT) END,
                        CASE WHEN priority_output_price IS NOT NULL THEN CAST(ROUND(priority_output_price * 1000000000) AS BIGINT) END,
                        CASE WHEN audio_input_price IS NOT NULL THEN CAST(ROUND(audio_input_price * 1000000000) AS BIGINT) END,
                        NULL, NULL,
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
                        model, 'USD',
                        ROUND(input_price * 1000000000)::BIGINT,
                        ROUND(output_price * 1000000000)::BIGINT,
                        CASE WHEN cache_read_price IS NOT NULL THEN ROUND(cache_read_price * 1000000000)::BIGINT END,
                        CASE WHEN cache_creation_price IS NOT NULL THEN ROUND(cache_creation_price * 1000000000)::BIGINT END,
                        CASE WHEN batch_input_price IS NOT NULL THEN ROUND(batch_input_price * 1000000000)::BIGINT END,
                        CASE WHEN batch_output_price IS NOT NULL THEN ROUND(batch_output_price * 1000000000)::BIGINT END,
                        CASE WHEN priority_input_price IS NOT NULL THEN ROUND(priority_input_price * 1000000000)::BIGINT END,
                        CASE WHEN priority_output_price IS NOT NULL THEN ROUND(priority_output_price * 1000000000)::BIGINT END,
                        CASE WHEN audio_input_price IS NOT NULL THEN ROUND(audio_input_price * 1000000000)::BIGINT END,
                        NULL, NULL,
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

        // Migration: Convert REAL price columns to BIGINT nanodollars (u64 precision migration)
        // Check if prices_v2.input_price is REAL (contains decimal point) or BIGINT
        // If REAL, migrate by multiplying by 10^9
        let needs_price_migration: bool = if kind == "sqlite" {
            // Check column type in SQLite
            let check_type: Option<String> = sqlx::query_scalar(
                "SELECT typeof(input_price) FROM prices_v2 LIMIT 1"
            )
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            // If we get 'real', we need to migrate
            check_type.as_deref() == Some("real")
        } else if kind == "postgres" {
            // Check if column type is double precision vs bigint
            let check_type: Option<String> = sqlx::query_scalar(
                "SELECT data_type FROM information_schema.columns WHERE table_name = 'prices_v2' AND column_name = 'input_price'"
            )
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            check_type.as_deref() == Some("double precision")
        } else {
            false
        };

        if needs_price_migration {
            println!("Migrating price columns from REAL to BIGINT nanodollars...");

            if kind == "sqlite" {
                // SQLite migration: Create new table, copy data, drop old, rename
                // Migrate prices_v2
                let _ = sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS prices_v2_new (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        model TEXT NOT NULL,
                        currency TEXT NOT NULL DEFAULT 'USD',
                        input_price BIGINT NOT NULL DEFAULT 0,
                        output_price BIGINT NOT NULL DEFAULT 0,
                        cache_read_input_price BIGINT,
                        cache_creation_input_price BIGINT,
                        batch_input_price BIGINT,
                        batch_output_price BIGINT,
                        priority_input_price BIGINT,
                        priority_output_price BIGINT,
                        audio_input_price BIGINT,
                        source TEXT,
                        region TEXT,
                        context_window INTEGER,
                        max_output_tokens INTEGER,
                        supports_vision INTEGER DEFAULT 0,
                        supports_function_calling INTEGER DEFAULT 0,
                        synced_at INTEGER,
                        created_at INTEGER,
                        updated_at INTEGER,
                        UNIQUE(model, currency, region)
                    )
                    "#,
                )
                .execute(pool)
                .await;

                let _ = sqlx::query(
                    r#"
                    INSERT INTO prices_v2_new
                    SELECT id, model, currency,
                           CAST(ROUND(input_price * 1000000000) AS BIGINT),
                           CAST(ROUND(output_price * 1000000000) AS BIGINT),
                           CASE WHEN cache_read_input_price IS NOT NULL THEN CAST(ROUND(cache_read_input_price * 1000000000) AS BIGINT) END,
                           CASE WHEN cache_creation_input_price IS NOT NULL THEN CAST(ROUND(cache_creation_input_price * 1000000000) AS BIGINT) END,
                           CASE WHEN batch_input_price IS NOT NULL THEN CAST(ROUND(batch_input_price * 1000000000) AS BIGINT) END,
                           CASE WHEN batch_output_price IS NOT NULL THEN CAST(ROUND(batch_output_price * 1000000000) AS BIGINT) END,
                           CASE WHEN priority_input_price IS NOT NULL THEN CAST(ROUND(priority_input_price * 1000000000) AS BIGINT) END,
                           CASE WHEN priority_output_price IS NOT NULL THEN CAST(ROUND(priority_output_price * 1000000000) AS BIGINT) END,
                           CASE WHEN audio_input_price IS NOT NULL THEN CAST(ROUND(audio_input_price * 1000000000) AS BIGINT) END,
                           source, region, context_window, max_output_tokens,
                           supports_vision, supports_function_calling, synced_at, created_at, updated_at
                    FROM prices_v2
                    "#,
                )
                .execute(pool)
                .await;

                let _ = sqlx::query("DROP TABLE prices_v2").execute(pool).await;
                let _ = sqlx::query("ALTER TABLE prices_v2_new RENAME TO prices_v2").execute(pool).await;
                let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_prices_v2_model ON prices_v2(model)").execute(pool).await;
                let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_prices_v2_model_currency ON prices_v2(model, currency)").execute(pool).await;

                // Migrate tiered_pricing
                let _ = sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS tiered_pricing_new (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        model TEXT NOT NULL,
                        region TEXT,
                        tier_start INTEGER NOT NULL,
                        tier_end INTEGER,
                        input_price BIGINT NOT NULL,
                        output_price BIGINT NOT NULL,
                        UNIQUE(model, region, tier_start)
                    )
                    "#,
                )
                .execute(pool)
                .await;

                let _ = sqlx::query(
                    r#"
                    INSERT INTO tiered_pricing_new
                    SELECT id, model, region, tier_start, tier_end,
                           CAST(ROUND(input_price * 1000000000) AS BIGINT),
                           CAST(ROUND(output_price * 1000000000) AS BIGINT)
                    FROM tiered_pricing
                    "#,
                )
                .execute(pool)
                .await;

                let _ = sqlx::query("DROP TABLE tiered_pricing").execute(pool).await;
                let _ = sqlx::query("ALTER TABLE tiered_pricing_new RENAME TO tiered_pricing").execute(pool).await;
                let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_tiered_pricing_model ON tiered_pricing(model)").execute(pool).await;

                // Migrate exchange_rates
                let _ = sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS exchange_rates_new (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        from_currency TEXT NOT NULL,
                        to_currency TEXT NOT NULL,
                        rate BIGINT NOT NULL,
                        updated_at INTEGER,
                        UNIQUE(from_currency, to_currency)
                    )
                    "#,
                )
                .execute(pool)
                .await;

                let _ = sqlx::query(
                    r#"
                    INSERT INTO exchange_rates_new
                    SELECT id, from_currency, to_currency,
                           CAST(ROUND(rate * 1000000000) AS BIGINT),
                           updated_at
                    FROM exchange_rates
                    "#,
                )
                .execute(pool)
                .await;

                let _ = sqlx::query("DROP TABLE exchange_rates").execute(pool).await;
                let _ = sqlx::query("ALTER TABLE exchange_rates_new RENAME TO exchange_rates").execute(pool).await;
                let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_exchange_rates_from ON exchange_rates(from_currency)").execute(pool).await;

                println!("SQLite price migration completed");
            } else if kind == "postgres" {
                // PostgreSQL migration: ALTER COLUMN with USING clause
                let _ = sqlx::query(
                    "ALTER TABLE prices_v2
                    ALTER COLUMN input_price TYPE BIGINT USING ROUND(input_price * 1000000000)::BIGINT,
                    ALTER COLUMN output_price TYPE BIGINT USING ROUND(output_price * 1000000000)::BIGINT"
                )
                .execute(pool)
                .await;

                // Migrate optional columns
                let optional_price_columns = [
                    "cache_read_input_price",
                    "cache_creation_input_price",
                    "batch_input_price",
                    "batch_output_price",
                    "priority_input_price",
                    "priority_output_price",
                    "audio_input_price",
                ];

                for column in optional_price_columns {
                    let _ = sqlx::query(&format!(
                        "ALTER TABLE prices_v2 ALTER COLUMN {} TYPE BIGINT USING ROUND({} * 1000000000)::BIGINT",
                        column, column
                    ))
                    .execute(pool)
                    .await;
                }

                // Migrate tiered_pricing
                let _ = sqlx::query(
                    "ALTER TABLE tiered_pricing
                    ALTER COLUMN input_price TYPE BIGINT USING ROUND(input_price * 1000000000)::BIGINT,
                    ALTER COLUMN output_price TYPE BIGINT USING ROUND(output_price * 1000000000)::BIGINT"
                )
                .execute(pool)
                .await;

                // Migrate exchange_rates
                let _ = sqlx::query(
                    "ALTER TABLE exchange_rates
                    ALTER COLUMN rate TYPE BIGINT USING ROUND(rate * 1000000000)::BIGINT"
                )
                .execute(pool)
                .await;

                println!("PostgreSQL price migration completed");
            }
        }

        // Migration: Change prices_v2 unique constraint from UNIQUE(model, currency, region) to UNIQUE(model, region)
        // This enforces the core principle: one model in one region can only have one currency pricing
        // Check if the constraint needs to be changed by looking for duplicates
        let needs_constraint_migration: bool = if kind == "sqlite" {
            // Check if we have the old constraint by trying to insert a duplicate
            // If the table has UNIQUE(model, currency, region), we need to migrate
            let constraint_check: Option<i64> = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pragma_index_list('prices_v2') WHERE name LIKE '%currency%'"
            )
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            constraint_check.unwrap_or(0) > 0
        } else if kind == "postgres" {
            // Check if the old constraint exists
            let constraint_check: Option<String> = sqlx::query_scalar(
                "SELECT constraint_name FROM information_schema.table_constraints
                 WHERE table_name = 'prices_v2' AND constraint_type = 'UNIQUE'
                 AND constraint_name LIKE '%currency%'"
            )
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            constraint_check.is_some()
        } else {
            false
        };

        if needs_constraint_migration {
            println!("Migrating prices_v2 unique constraint to UNIQUE(model, region)...");

            if kind == "sqlite" {
                // SQLite: Create new table with correct constraint, migrate data, swap tables
                // First, clean duplicates - keep only one row per (model, region)
                // We keep the row with the lowest id for each (model, region) combination
                let _ = sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS prices_v2_new (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        model TEXT NOT NULL,
                        currency TEXT NOT NULL DEFAULT 'USD',
                        input_price BIGINT NOT NULL DEFAULT 0,
                        output_price BIGINT NOT NULL DEFAULT 0,
                        cache_read_input_price BIGINT,
                        cache_creation_input_price BIGINT,
                        batch_input_price BIGINT,
                        batch_output_price BIGINT,
                        priority_input_price BIGINT,
                        priority_output_price BIGINT,
                        audio_input_price BIGINT,
                        source TEXT,
                        region TEXT,
                        context_window INTEGER,
                        max_output_tokens INTEGER,
                        supports_vision INTEGER DEFAULT 0,
                        supports_function_calling INTEGER DEFAULT 0,
                        synced_at INTEGER,
                        created_at INTEGER,
                        updated_at INTEGER,
                        UNIQUE(model, region)
                    )
                    "#,
                )
                .execute(pool)
                .await;

                // Insert deduplicated data - keep first entry for each (model, region)
                let _ = sqlx::query(
                    r#"
                    INSERT INTO prices_v2_new
                    SELECT id, model, currency, input_price, output_price,
                           cache_read_input_price, cache_creation_input_price,
                           batch_input_price, batch_output_price,
                           priority_input_price, priority_output_price,
                           audio_input_price, source, region,
                           context_window, max_output_tokens,
                           supports_vision, supports_function_calling,
                           synced_at, created_at, updated_at
                    FROM prices_v2
                    WHERE id IN (
                        SELECT MIN(id) FROM prices_v2 GROUP BY model, region
                    )
                    "#,
                )
                .execute(pool)
                .await;

                let _ = sqlx::query("DROP TABLE prices_v2").execute(pool).await;
                let _ = sqlx::query("ALTER TABLE prices_v2_new RENAME TO prices_v2").execute(pool).await;
                let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_prices_v2_model ON prices_v2(model)").execute(pool).await;
                let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_prices_v2_model_region ON prices_v2(model, region)").execute(pool).await;

                println!("SQLite prices_v2 constraint migration completed");
            } else if kind == "postgres" {
                // PostgreSQL: Drop old constraint and add new one
                // First, clean duplicates
                let _ = sqlx::query(
                    "DELETE FROM prices_v2 WHERE id NOT IN (
                        SELECT MIN(id) FROM prices_v2 GROUP BY model, region
                    )"
                )
                .execute(pool)
                .await;

                // Drop the old constraint (name varies, so we find it dynamically)
                let _ = sqlx::query(
                    "DO $$
                    DECLARE constraint_name TEXT;
                    BEGIN
                        SELECT c.conname INTO constraint_name
                        FROM pg_constraint c
                        JOIN pg_class t ON c.conrelid = t.oid
                        WHERE t.relname = 'prices_v2'
                        AND c.contype = 'u'
                        AND EXISTS (
                            SELECT 1 FROM pg_attribute a
                            WHERE a.attrelid = t.oid
                            AND a.attnum = ANY(c.conkey)
                            AND a.attname = 'currency'
                        );
                        IF constraint_name IS NOT NULL THEN
                            EXECUTE 'ALTER TABLE prices_v2 DROP CONSTRAINT ' || constraint_name;
                        END IF;
                    END $$;"
                )
                .execute(pool)
                .await;

                // Add the new constraint
                let _ = sqlx::query(
                    "ALTER TABLE prices_v2 ADD CONSTRAINT prices_v2_model_region_unique UNIQUE (model, region)"
                )
                .execute(pool)
                .await;

                // Create the new index
                let _ = sqlx::query(
                    "CREATE INDEX IF NOT EXISTS idx_prices_v2_model_region ON prices_v2(model, region)"
                )
                .execute(pool)
                .await;

                println!("PostgreSQL prices_v2 constraint migration completed");
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
