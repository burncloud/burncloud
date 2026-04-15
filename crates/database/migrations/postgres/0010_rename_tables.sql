-- Migration 0010: Create canonical table names (PostgreSQL)
-- All tables renamed per the three-dimension naming convention:
--   table = {domain}_{entities}  (snake_case, plural)
-- Data is copied from legacy names in schema/rename.rs -- these CREATE TABLE
-- statements ensure fresh installs also get the canonical names.
-- Idempotent: all statements use CREATE TABLE IF NOT EXISTS / CREATE INDEX IF NOT EXISTS.
--
-- Note: "group" is a reserved SQL keyword; quoted with double-quotes for PostgreSQL.

-- ─── user_ domain ───────────────────────────────────────────────────────────

-- 1. user_accounts  (replaces: users)
CREATE TABLE IF NOT EXISTS user_accounts (
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
CREATE INDEX IF NOT EXISTS idx_user_accounts_username ON user_accounts(username);
CREATE INDEX IF NOT EXISTS idx_user_accounts_email ON user_accounts(email);

-- 2. user_roles  (replaces: roles)
CREATE TABLE IF NOT EXISTS user_roles (
    id TEXT PRIMARY KEY,
    name VARCHAR(191) NOT NULL UNIQUE,
    description TEXT
);

-- 3. user_role_bindings  (replaces: user_roles binding table)
--    Created after user_accounts and user_roles so FK references resolve.
CREATE TABLE IF NOT EXISTS user_role_bindings (
    user_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    PRIMARY KEY (user_id, role_id),
    FOREIGN KEY(user_id) REFERENCES user_accounts(id) ON DELETE CASCADE,
    FOREIGN KEY(role_id) REFERENCES user_roles(id) ON DELETE CASCADE
);

-- 4. user_recharges  (replaces: recharges)
CREATE TABLE IF NOT EXISTS user_recharges (
    id SERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    amount BIGINT NOT NULL,
    currency VARCHAR(10) DEFAULT 'USD',
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(user_id) REFERENCES user_accounts(id) ON DELETE CASCADE
);

-- 5. user_api_keys  (replaces: tokens)
CREATE TABLE IF NOT EXISTS user_api_keys (
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
CREATE INDEX IF NOT EXISTS idx_user_api_keys_key ON user_api_keys(key);
CREATE INDEX IF NOT EXISTS idx_user_api_keys_user_id ON user_api_keys(user_id);

-- ─── channel_ domain ────────────────────────────────────────────────────────

-- 6. channel_providers  (replaces: channels)
CREATE TABLE IF NOT EXISTS channel_providers (
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
CREATE INDEX IF NOT EXISTS idx_channel_providers_name ON channel_providers(name);
CREATE INDEX IF NOT EXISTS idx_channel_providers_tag ON channel_providers(tag);

-- 7. channel_abilities  (replaces: abilities)
CREATE TABLE IF NOT EXISTS channel_abilities (
    "group" VARCHAR(64) NOT NULL,
    model VARCHAR(255) NOT NULL,
    channel_id INTEGER NOT NULL,
    enabled BOOLEAN DEFAULT TRUE,
    priority BIGINT DEFAULT 0,
    weight INTEGER DEFAULT 0,
    tag VARCHAR(30),
    PRIMARY KEY ("group", model, channel_id)
);
CREATE INDEX IF NOT EXISTS idx_channel_abilities_model ON channel_abilities(model);
CREATE INDEX IF NOT EXISTS idx_channel_abilities_channel_id ON channel_abilities(channel_id);

-- 8. channel_protocol_configs  (replaces: protocol_configs)
CREATE TABLE IF NOT EXISTS channel_protocol_configs (
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
CREATE INDEX IF NOT EXISTS idx_channel_protocol_configs_type ON channel_protocol_configs(channel_type);
CREATE INDEX IF NOT EXISTS idx_channel_protocol_configs_version ON channel_protocol_configs(api_version);

-- ─── billing_ domain ────────────────────────────────────────────────────────

-- 9. billing_prices  (replaces: prices)
CREATE TABLE IF NOT EXISTS billing_prices (
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
CREATE INDEX IF NOT EXISTS idx_billing_prices_model ON billing_prices(model);
CREATE INDEX IF NOT EXISTS idx_billing_prices_model_region ON billing_prices(model, region);

-- 10. billing_tiered_prices  (replaces: tiered_pricing)
CREATE TABLE IF NOT EXISTS billing_tiered_prices (
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
CREATE INDEX IF NOT EXISTS idx_billing_tiered_prices_model ON billing_tiered_prices(model);

-- 11. billing_exchange_rates  (replaces: exchange_rates)
CREATE TABLE IF NOT EXISTS billing_exchange_rates (
    id SERIAL PRIMARY KEY,
    from_currency VARCHAR(10) NOT NULL,
    to_currency VARCHAR(10) NOT NULL,
    rate BIGINT NOT NULL,
    updated_at BIGINT,
    UNIQUE(from_currency, to_currency)
);
CREATE INDEX IF NOT EXISTS idx_billing_exchange_rates_from ON billing_exchange_rates(from_currency);

-- ─── router_ domain ─────────────────────────────────────────────────────────

-- 12. router_video_tasks  (replaces: video_tasks)
CREATE TABLE IF NOT EXISTS router_video_tasks (
    task_id TEXT PRIMARY KEY,
    channel_id INTEGER NOT NULL,
    user_id TEXT,
    model TEXT,
    duration INTEGER DEFAULT 5,
    resolution TEXT DEFAULT '720p',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_router_video_tasks_created_at ON router_video_tasks(created_at);

-- ─── sys_ domain ────────────────────────────────────────────────────────────

-- 13. sys_settings  (replaces: setting)
CREATE TABLE IF NOT EXISTS sys_settings (
    name TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- 14. sys_downloads  (replaces: downloads)
CREATE TABLE IF NOT EXISTS sys_downloads (
    gid TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'waiting',
    uris TEXT NOT NULL,
    total_length BIGINT DEFAULT 0,
    completed_length BIGINT DEFAULT 0,
    download_speed BIGINT DEFAULT 0,
    download_dir TEXT,
    filename TEXT,
    connections INTEGER DEFAULT 16,
    split INTEGER DEFAULT 5,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_sys_downloads_status ON sys_downloads(status);

-- 15. sys_installations  (replaces: installations)
CREATE TABLE IF NOT EXISTS sys_installations (
    software_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT,
    status TEXT NOT NULL DEFAULT 'not_installed',
    install_dir TEXT,
    install_method TEXT,
    installed_at TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    error_message TEXT
);
CREATE INDEX IF NOT EXISTS idx_sys_installations_status ON sys_installations(status);
CREATE INDEX IF NOT EXISTS idx_sys_installations_installed_at ON sys_installations(installed_at)
