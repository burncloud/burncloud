use crate::{Database, Result};
use sqlx::Executor;

/// Get current Unix timestamp in seconds
/// Returns 0 if system time is before Unix epoch (extremely unlikely)
fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

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
        // Note: unlimited_quota uses INTEGER (0/1) for SQLite compatibility (SQLite has no native BOOLEAN)
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
                    unlimited_quota INTEGER DEFAULT 0,
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
                    unlimited_quota INTEGER DEFAULT 0,
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

        // 4.5. Router Logs Table (API Request Logging)
        // ============================================================================
        // Stores logs of all API requests through the router
        // - cost stored as BIGINT nanodollars (9 decimal precision)
        // - created_at as TEXT for SQLite compatibility (avoid DATETIME type issues with sqlx Any driver)
        // ============================================================================
        let router_logs_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS router_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    request_id TEXT NOT NULL,
                    user_id TEXT,
                    path TEXT NOT NULL,
                    upstream_id TEXT,
                    status_code INTEGER NOT NULL,
                    latency_ms INTEGER NOT NULL,
                    prompt_tokens INTEGER DEFAULT 0,
                    completion_tokens INTEGER DEFAULT 0,
                    cost INTEGER DEFAULT 0,
                    created_at TEXT DEFAULT CURRENT_TIMESTAMP
                );
                CREATE INDEX IF NOT EXISTS idx_router_logs_user_id ON router_logs(user_id);
                CREATE INDEX IF NOT EXISTS idx_router_logs_created_at ON router_logs(created_at);
            "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS router_logs (
                    id SERIAL PRIMARY KEY,
                    request_id TEXT NOT NULL,
                    user_id TEXT,
                    path TEXT NOT NULL,
                    upstream_id TEXT,
                    status_code INTEGER NOT NULL,
                    latency_ms BIGINT NOT NULL,
                    prompt_tokens INTEGER DEFAULT 0,
                    completion_tokens INTEGER DEFAULT 0,
                    cost BIGINT DEFAULT 0,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
                CREATE INDEX IF NOT EXISTS idx_router_logs_user_id ON router_logs(user_id);
                CREATE INDEX IF NOT EXISTS idx_router_logs_created_at ON router_logs(created_at);
            "#
            }
            _ => "",
        };

        // 5. Prices Table (Model Pricing) with Advanced Pricing Fields
        // ============================================================================
        // NEW: Prices table now uses BIGINT nanodollars (9 decimal precision)
        // - All prices stored as BIGINT nanodollars (1 dollar = 1,000,000,000 nanodollars)
        // - Supports multi-currency pricing with regional support
        // - Unique constraint: UNIQUE(model, region) - one currency per region per model
        // - Old prices table (with REAL columns) has been migrated and renamed to prices_deprecated
        // ============================================================================
        let prices_sql = match kind.as_str() {
            "sqlite" => {
                r#"
                CREATE TABLE IF NOT EXISTS prices (
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
                CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model);
                CREATE INDEX IF NOT EXISTS idx_prices_model_region ON prices(model, region);
            "#
            }
            "postgres" => {
                r#"
                CREATE TABLE IF NOT EXISTS prices (
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
                    supports_vision INTEGER DEFAULT 0,
                    supports_function_calling INTEGER DEFAULT 0,
                    synced_at BIGINT,
                    created_at BIGINT,
                    updated_at BIGINT,
                    UNIQUE(model, region)
                );
                CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model);
                CREATE INDEX IF NOT EXISTS idx_prices_model_region ON prices(model, region);
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

        // 10. Migration: Rename prices_v2 to prices and cleanup deprecated tables
        // This section handles the migration from prices_v2 to prices
        // No new table creation - the prices table above is now the primary table
        // Note: prices_v2_sql removed - deprecated table, no longer needed

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
        if !router_logs_sql.is_empty() {
            pool.execute(router_logs_sql).await?;
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

        // Migration: Fix router_logs table schema for SQLite
        // The old schema used DATETIME and REAL which are incompatible with sqlx Any driver
        // SQLite doesn't support ALTER COLUMN, so we recreate the table
        if kind == "sqlite" {
            // Check if router_logs table exists
            let table_exists: bool = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='router_logs'",
            )
            .fetch_one(pool)
            .await
            .unwrap_or(0) > 0;

            // Check if migration is needed by examining column types via pragma_table_info
            // Use a raw query that doesn't try to decode any DATETIME columns
            let needs_migration = if table_exists {
                // Query the column type directly without decoding any data
                let col_type: Option<String> = sqlx::query_scalar(
                    "SELECT type FROM pragma_table_info('router_logs') WHERE name='created_at'",
                )
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

                col_type.as_ref().map(|t| t == "DATETIME").unwrap_or(false)
            } else {
                false
            };

            if needs_migration {
                eprintln!("[Migration] Migrating router_logs table: DATETIME -> TEXT, REAL -> INTEGER");

                // Drop temp table if exists from previous failed migration
                let _ = sqlx::query("DROP TABLE IF EXISTS router_logs_new")
                    .execute(pool)
                    .await;

                // Create new table with correct schema
                let _ = sqlx::query(
                    r#"
                    CREATE TABLE router_logs_new (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        request_id TEXT NOT NULL,
                        user_id TEXT,
                        path TEXT NOT NULL,
                        upstream_id TEXT,
                        status_code INTEGER NOT NULL,
                        latency_ms INTEGER NOT NULL,
                        prompt_tokens INTEGER DEFAULT 0,
                        completion_tokens INTEGER DEFAULT 0,
                        cost INTEGER DEFAULT 0,
                        created_at TEXT DEFAULT CURRENT_TIMESTAMP
                    )
                    "#,
                )
                .execute(pool)
                .await;

                // Copy data with type conversion
                let _ = sqlx::query(
                    "INSERT INTO router_logs_new SELECT id, request_id, user_id, path, upstream_id, status_code, latency_ms, prompt_tokens, completion_tokens, CAST(COALESCE(cost, 0) AS INTEGER), created_at FROM router_logs"
                )
                .execute(pool)
                .await;

                // Drop old table
                let _ = sqlx::query("DROP TABLE router_logs")
                    .execute(pool)
                    .await;

                // Rename new table
                let _ = sqlx::query("ALTER TABLE router_logs_new RENAME TO router_logs")
                    .execute(pool)
                    .await;

                // Recreate indexes
                let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_router_logs_user_id ON router_logs(user_id)")
                    .execute(pool)
                    .await;
                let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_router_logs_created_at ON router_logs(created_at)")
                    .execute(pool)
                    .await;

                eprintln!("[Migration] router_logs table migration completed successfully");
            }
        }

        // Migration: Migrate prices_v2 to prices and cleanup deprecated tables
        // This handles the transition from:
        //   - Old prices table (REAL columns) → prices_deprecated
        //   - prices_v2 (BIGINT columns) → prices (new primary table)
        //   - Cleanup: prices_v2_new, tiered_pricing_new, exchange_rates_new

        // Step 1: Check if prices_v2 exists and needs to be migrated to prices
        let prices_v2_exists: bool = if kind == "sqlite" {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='prices_v2'",
            )
            .fetch_one(pool)
            .await
            .unwrap_or(0);
            count > 0
        } else {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM information_schema.tables WHERE table_name = 'prices_v2'",
            )
            .fetch_one(pool)
            .await
            .unwrap_or(0);
            count > 0
        };

        if prices_v2_exists {
            println!("Migrating prices_v2 to prices table...");

            // Check if old prices table exists (with REAL columns)
            let old_prices_exists: bool = if kind == "sqlite" {
                let count: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='prices'",
                )
                .fetch_one(pool)
                .await
                .unwrap_or(0);
                count > 0
            } else {
                let count: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM information_schema.tables WHERE table_name = 'prices'",
                )
                .fetch_one(pool)
                .await
                .unwrap_or(0);
                count > 0
            };

            // Rename old prices table to prices_deprecated
            if old_prices_exists {
                let _ = sqlx::query("DROP TABLE IF EXISTS prices_deprecated")
                    .execute(pool)
                    .await;
                let _ = sqlx::query("ALTER TABLE prices RENAME TO prices_deprecated")
                    .execute(pool)
                    .await;
                println!("  Renamed old 'prices' table to 'prices_deprecated'");
            }

            // Rename prices_v2 to prices
            let _ = sqlx::query("ALTER TABLE prices_v2 RENAME TO prices")
                .execute(pool)
                .await;

            // Recreate indexes
            let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model)")
                .execute(pool)
                .await;
            let _ = sqlx::query(
                "CREATE INDEX IF NOT EXISTS idx_prices_model_region ON prices(model, region)",
            )
            .execute(pool)
            .await;

            println!("  Renamed 'prices_v2' to 'prices'");
        }

        // Step 2: Cleanup temporary migration tables
        let temp_tables = ["prices_v2_new", "tiered_pricing_new", "exchange_rates_new"];
        for table in temp_tables {
            if kind == "sqlite" {
                let exists: i64 = sqlx::query_scalar(&format!(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='{}'",
                    table
                ))
                .fetch_one(pool)
                .await
                .unwrap_or(0);

                if exists > 0 {
                    let _ = sqlx::query(&format!("DROP TABLE {}", table))
                        .execute(pool)
                        .await;
                    println!("  Dropped temporary table '{}'", table);
                }
            } else {
                let _ = sqlx::query(&format!("DROP TABLE IF EXISTS {}", table))
                    .execute(pool)
                    .await;
            }
        }

        // Step 3: Migrate data from prices_deprecated to prices (if prices is empty)
        let prices_count: i64 = sqlx::query_scalar("SELECT count(*) FROM prices")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        let deprecated_exists: bool = if kind == "sqlite" {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='prices_deprecated'"
            )
            .fetch_one(pool)
            .await
            .unwrap_or(0);
            count > 0
        } else {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM information_schema.tables WHERE table_name = 'prices_deprecated'"
            )
            .fetch_one(pool)
            .await
            .unwrap_or(0);
            count > 0
        };

        if prices_count == 0 && deprecated_exists {
            let now = current_timestamp();

            let migrate_sql = match kind.as_str() {
                "sqlite" => {
                    r#"
                    INSERT INTO prices (
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
                    FROM prices_deprecated
                "#
                }
                "postgres" => {
                    r#"
                    INSERT INTO prices (
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
                    FROM prices_deprecated
                    ON CONFLICT (model, region) DO NOTHING
                "#
                }
                _ => "",
            };

            if !migrate_sql.is_empty() {
                let _ = sqlx::query(migrate_sql)
                    .bind(now)
                    .bind(now)
                    .execute(pool)
                    .await;
                println!("  Migrated data from prices_deprecated to prices");
            }
        }

        // Step 4: Migrate unlimited_quota from BOOLEAN to INTEGER for SQLite compatibility
        // SQLite doesn't have native BOOLEAN, it stores as INTEGER but sqlx Any driver expects INTEGER type
        if kind == "sqlite" {
            // Check if unlimited_quota column type needs migration
            let needs_migration: bool = {
                let result: Option<String> =
                    sqlx::query_scalar("SELECT typeof(unlimited_quota) FROM tokens LIMIT 1")
                        .fetch_optional(pool)
                        .await
                        .unwrap_or(None);

                // If it returns 'boolean' or error, we need to recreate the table
                result.is_none() || result == Some("boolean".to_string())
            };

            if needs_migration {
                println!("Migrating tokens.unlimited_quota from BOOLEAN to INTEGER...");

                // Create new tokens table with correct schema
                let _ = sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS tokens_new (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        user_id TEXT NOT NULL,
                        key CHAR(48) NOT NULL,
                        status INTEGER DEFAULT 1,
                        name VARCHAR(255),
                        remain_quota INTEGER DEFAULT 0,
                        unlimited_quota INTEGER DEFAULT 0,
                        used_quota INTEGER DEFAULT 0,
                        created_time INTEGER,
                        accessed_time INTEGER,
                        expired_time INTEGER DEFAULT -1
                    )
                    "#,
                )
                .execute(pool)
                .await;

                // Copy data
                let _ = sqlx::query(
                    r#"
                    INSERT INTO tokens_new
                    SELECT id, user_id, key, status, name, remain_quota,
                           CASE WHEN unlimited_quota = 1 OR unlimited_quota = 'true' THEN 1 ELSE 0 END,
                           used_quota, created_time, accessed_time, expired_time
                    FROM tokens
                    "#,
                )
                .execute(pool)
                .await;

                // Drop old and rename
                let _ = sqlx::query("DROP TABLE tokens").execute(pool).await;
                let _ = sqlx::query("ALTER TABLE tokens_new RENAME TO tokens")
                    .execute(pool)
                    .await;
                let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_tokens_key ON tokens(key)")
                    .execute(pool)
                    .await;
                let _ =
                    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tokens_user_id ON tokens(user_id)")
                        .execute(pool)
                        .await;

                println!("  Migrated tokens table schema");
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
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN balance_usd BIGINT DEFAULT 0")
                .execute(pool)
                .await;
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN balance_cny BIGINT DEFAULT 0")
                .execute(pool)
                .await;
        } else if kind == "postgres" {
            let _ = sqlx::query(
                "ALTER TABLE users ADD COLUMN IF NOT EXISTS balance_usd BIGINT DEFAULT 0",
            )
            .execute(pool)
            .await;
            let _ = sqlx::query(
                "ALTER TABLE users ADD COLUMN IF NOT EXISTS balance_cny BIGINT DEFAULT 0",
            )
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
            let now = current_timestamp();
            let insert_token_sql = match kind.as_str() {
                "sqlite" => "INSERT INTO tokens (user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time) VALUES ('demo-user', 'sk-burncloud-demo', 1, 'Demo Token', -1, 1, 0, ?, ?, -1)",
                "postgres" => "INSERT INTO tokens (user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time) VALUES ('demo-user', 'sk-burncloud-demo', 1, 'Demo Token', -1, 1, 0, $1, $2, -1)",
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
            let now = current_timestamp();

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
            let now = current_timestamp();

            // Type alias for protocol config: (channel_type, api_version, is_default, chat_endpoint, embed_endpoint, models_endpoint)
            type ProtocolConfig<'a> = (
                i32,
                &'a str,
                bool,
                Option<&'a str>,
                Option<&'a str>,
                Option<&'a str>,
            );

            // Default protocol configs
            // channel_type values: 1=OpenAI, 2=Anthropic, 3=Azure, 4=Gemini, 5=Vertex
            let default_protocols: [ProtocolConfig; 4] = [
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
