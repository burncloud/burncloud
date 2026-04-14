-- Migration 0001: Initial schema (all core tables, current column set)
-- Idempotent: all statements use CREATE TABLE IF NOT EXISTS / CREATE INDEX IF NOT EXISTS

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
    api_version VARCHAR(32) DEFAULT 'default',
    pricing_region VARCHAR(32) DEFAULT 'international'
);
CREATE INDEX IF NOT EXISTS idx_channels_name ON channels(name);
CREATE INDEX IF NOT EXISTS idx_channels_tag ON channels(tag);

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
    input_cost INTEGER DEFAULT 0,
    output_cost INTEGER DEFAULT 0,
    cache_read_cost INTEGER DEFAULT 0,
    cache_write_cost INTEGER DEFAULT 0,
    audio_cost INTEGER DEFAULT 0,
    image_cost INTEGER DEFAULT 0,
    video_cost INTEGER DEFAULT 0,
    reasoning_cost INTEGER DEFAULT 0,
    embedding_cost INTEGER DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_router_logs_user_id ON router_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_router_logs_created_at ON router_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_router_logs_model ON router_logs(model);

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
    audio_output_price BIGINT,
    reasoning_price BIGINT,
    embedding_price BIGINT,
    image_price BIGINT,
    video_price BIGINT,
    music_price BIGINT,
    alias_for TEXT,
    source TEXT,
    region TEXT NOT NULL DEFAULT '',
    context_window INTEGER,
    max_output_tokens INTEGER,
    supports_vision INTEGER DEFAULT 0,
    supports_function_calling INTEGER DEFAULT 0,
    synced_at INTEGER,
    created_at INTEGER,
    updated_at INTEGER,
    voices_pricing TEXT,
    video_pricing TEXT,
    asr_pricing TEXT,
    realtime_pricing TEXT,
    model_type TEXT,
    UNIQUE(model, region)
);
CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model);
CREATE INDEX IF NOT EXISTS idx_prices_model_region ON prices(model, region);

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

CREATE TABLE IF NOT EXISTS tiered_pricing (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model TEXT NOT NULL,
    region TEXT,
    tier_start INTEGER NOT NULL,
    tier_end INTEGER,
    input_price BIGINT NOT NULL,
    output_price BIGINT NOT NULL,
    currency VARCHAR(10) DEFAULT 'USD',
    tier_type VARCHAR(32) DEFAULT 'context_length',
    UNIQUE(model, region, tier_start)
);
CREATE INDEX IF NOT EXISTS idx_tiered_pricing_model ON tiered_pricing(model);

CREATE TABLE IF NOT EXISTS exchange_rates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_currency TEXT NOT NULL,
    to_currency TEXT NOT NULL,
    rate BIGINT NOT NULL,
    updated_at INTEGER,
    UNIQUE(from_currency, to_currency)
);
CREATE INDEX IF NOT EXISTS idx_exchange_rates_from ON exchange_rates(from_currency);

CREATE TABLE IF NOT EXISTS video_tasks (
    task_id TEXT PRIMARY KEY,
    channel_id INTEGER NOT NULL,
    user_id TEXT,
    model TEXT,
    duration INTEGER DEFAULT 5,
    resolution TEXT DEFAULT '720p',
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_video_tasks_created_at ON video_tasks(created_at)
