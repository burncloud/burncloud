-- Migration 0001: Initial schema for BurnCloud (PostgreSQL)
--
-- Creates all core tables with the current full schema.
-- All subsequent ALTER TABLE changes are folded into the column definitions here,
-- so a fresh install gets the complete up-to-date schema in one shot.
--
-- Note: "group" is a reserved SQL keyword; quoted with double-quotes for PostgreSQL.
-- Note: balance_usd / balance_cny store nanodollars (i64, 10^9 precision).

-- 1. Users
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

-- 2. Channels (includes api_version and pricing_region from later ALTER TABLE migrations)
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
    api_version VARCHAR(32) DEFAULT 'default',
    pricing_region VARCHAR(32) DEFAULT 'international'
);
CREATE INDEX IF NOT EXISTS idx_channels_name ON channels(name);
CREATE INDEX IF NOT EXISTS idx_channels_tag ON channels(tag);

-- 3. Abilities (routing core; composite primary key)
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

-- 4. Tokens (app-level API tokens)
-- remain_quota / used_quota: deprecated in favour of user-level dual-currency wallet;
--   retained for backward compatibility.
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

-- 5. Router logs (all columns, including those added in later ALTER TABLE migrations)
-- cost / *_cost columns store BIGINT nanodollars.
-- Postgres: cost columns must be BIGINT (int4 overflows at ~$2.10/request).
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
    model TEXT,
    cache_read_tokens INTEGER DEFAULT 0,
    reasoning_tokens INTEGER DEFAULT 0,
    pricing_region TEXT DEFAULT 'international',
    video_tokens INTEGER DEFAULT 0,
    cache_write_tokens INTEGER DEFAULT 0,
    audio_input_tokens INTEGER DEFAULT 0,
    audio_output_tokens INTEGER DEFAULT 0,
    image_tokens INTEGER DEFAULT 0,
    embedding_tokens INTEGER DEFAULT 0,
    input_cost BIGINT DEFAULT 0,
    output_cost BIGINT DEFAULT 0,
    cache_read_cost BIGINT DEFAULT 0,
    cache_write_cost BIGINT DEFAULT 0,
    audio_cost BIGINT DEFAULT 0,
    image_cost BIGINT DEFAULT 0,
    video_cost BIGINT DEFAULT 0,
    reasoning_cost BIGINT DEFAULT 0,
    embedding_cost BIGINT DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_router_logs_user_id ON router_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_router_logs_created_at ON router_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_router_logs_model ON router_logs(model);

-- 6. Prices (BIGINT nanodollars; includes all multimodal and extended columns)
-- UNIQUE(model, region): one price entry per model per region.
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
    audio_output_price BIGINT,
    reasoning_price BIGINT,
    embedding_price BIGINT,
    image_price BIGINT,
    video_price BIGINT,
    music_price BIGINT,
    alias_for VARCHAR(255),
    source VARCHAR(64),
    region VARCHAR(32) NOT NULL DEFAULT '',
    context_window BIGINT,
    max_output_tokens BIGINT,
    supports_vision INTEGER DEFAULT 0,
    supports_function_calling INTEGER DEFAULT 0,
    synced_at BIGINT,
    created_at BIGINT,
    updated_at BIGINT,
    voices_pricing TEXT,
    video_pricing TEXT,
    asr_pricing TEXT,
    realtime_pricing TEXT,
    model_type VARCHAR(32),
    UNIQUE(model, region)
);
CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model);
CREATE INDEX IF NOT EXISTS idx_prices_model_region ON prices(model, region);

-- 7. Protocol configs (dynamic adapter configuration)
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

-- 8. Model capabilities (synced from LiteLLM)
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

-- 9. Tiered pricing (includes currency and tier_type from later ALTER TABLE migrations)
-- Prices stored as BIGINT nanodollars.
CREATE TABLE IF NOT EXISTS tiered_pricing (
    id SERIAL PRIMARY KEY,
    model VARCHAR(255) NOT NULL,
    region VARCHAR(32),
    tier_start BIGINT NOT NULL,
    tier_end BIGINT,
    input_price BIGINT NOT NULL,
    output_price BIGINT NOT NULL,
    currency VARCHAR(10) DEFAULT 'USD',
    tier_type VARCHAR(32) DEFAULT 'context_length',
    UNIQUE(model, region, tier_start)
);
CREATE INDEX IF NOT EXISTS idx_tiered_pricing_model ON tiered_pricing(model);

-- 10. Exchange rates (rate stored as BIGINT scaled by 10^9)
CREATE TABLE IF NOT EXISTS exchange_rates (
    id SERIAL PRIMARY KEY,
    from_currency VARCHAR(10) NOT NULL,
    to_currency VARCHAR(10) NOT NULL,
    rate BIGINT NOT NULL,
    updated_at BIGINT,
    UNIQUE(from_currency, to_currency)
);
CREATE INDEX IF NOT EXISTS idx_exchange_rates_from ON exchange_rates(from_currency);

-- 11. Video tasks (async video generation task_id → channel_id mapping)
CREATE TABLE IF NOT EXISTS video_tasks (
    task_id TEXT PRIMARY KEY,
    channel_id INTEGER NOT NULL,
    user_id TEXT,
    model TEXT,
    duration INTEGER DEFAULT 5,
    resolution TEXT DEFAULT '720p',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_video_tasks_created_at ON video_tasks(created_at);
