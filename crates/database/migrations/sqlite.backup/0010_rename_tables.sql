-- Migration 0010: Create canonical table names (SQLite)
-- All tables renamed per the three-dimension naming convention:
--   table = {domain}_{entities}  (snake_case, plural)
-- Data is copied from legacy names in schema/rename.rs -- these CREATE TABLE
-- statements ensure fresh installs also get the canonical names.
-- Idempotent: all statements use CREATE TABLE IF NOT EXISTS / CREATE INDEX IF NOT EXISTS.

-- ─── user_ domain ───────────────────────────────────────────────────────────

-- 1. user_accounts  (replaces: users)
CREATE TABLE IF NOT EXISTS user_accounts (
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
CREATE INDEX IF NOT EXISTS idx_user_accounts_username ON user_accounts(username);
CREATE INDEX IF NOT EXISTS idx_user_accounts_email ON user_accounts(email);

-- 2. user_roles  (replaces: roles)
CREATE TABLE IF NOT EXISTS user_roles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT
);

-- 3. user_role_bindings  (replaces: user_roles binding table)
--    Created after user_accounts and user_roles so FK references are resolvable.
CREATE TABLE IF NOT EXISTS user_role_bindings (
    user_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    PRIMARY KEY (user_id, role_id),
    FOREIGN KEY(user_id) REFERENCES user_accounts(id) ON DELETE CASCADE,
    FOREIGN KEY(role_id) REFERENCES user_roles(id) ON DELETE CASCADE
);

-- 4. user_recharges  (replaces: recharges)
CREATE TABLE IF NOT EXISTS user_recharges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    amount BIGINT NOT NULL,
    currency VARCHAR(10) DEFAULT 'USD',
    description TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(user_id) REFERENCES user_accounts(id) ON DELETE CASCADE
);

-- 5. user_api_keys  (replaces: tokens)
CREATE TABLE IF NOT EXISTS user_api_keys (
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
CREATE INDEX IF NOT EXISTS idx_user_api_keys_key ON user_api_keys(key);
CREATE INDEX IF NOT EXISTS idx_user_api_keys_user_id ON user_api_keys(user_id);

-- ─── channel_ domain ────────────────────────────────────────────────────────

-- 6. channel_providers  (replaces: channels)
CREATE TABLE IF NOT EXISTS channel_providers (
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
CREATE INDEX IF NOT EXISTS idx_channel_providers_name ON channel_providers(name);
CREATE INDEX IF NOT EXISTS idx_channel_providers_tag ON channel_providers(tag);

-- 7. channel_abilities  (replaces: abilities)
CREATE TABLE IF NOT EXISTS channel_abilities (
    `group` VARCHAR(64) NOT NULL,
    model VARCHAR(255) NOT NULL,
    channel_id INTEGER NOT NULL,
    enabled BOOLEAN DEFAULT 1,
    priority INTEGER DEFAULT 0,
    weight INTEGER DEFAULT 0,
    tag TEXT,
    PRIMARY KEY (`group`, model, channel_id)
);
CREATE INDEX IF NOT EXISTS idx_channel_abilities_model ON channel_abilities(model);
CREATE INDEX IF NOT EXISTS idx_channel_abilities_channel_id ON channel_abilities(channel_id);

-- 8. channel_protocol_configs  (replaces: protocol_configs)
CREATE TABLE IF NOT EXISTS channel_protocol_configs (
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
CREATE INDEX IF NOT EXISTS idx_channel_protocol_configs_type ON channel_protocol_configs(channel_type);
CREATE INDEX IF NOT EXISTS idx_channel_protocol_configs_version ON channel_protocol_configs(api_version);

-- ─── billing_ domain ────────────────────────────────────────────────────────

-- 9. billing_prices  (replaces: prices)
CREATE TABLE IF NOT EXISTS billing_prices (
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
CREATE INDEX IF NOT EXISTS idx_billing_prices_model ON billing_prices(model);
CREATE INDEX IF NOT EXISTS idx_billing_prices_model_region ON billing_prices(model, region);

-- 10. billing_tiered_prices  (replaces: tiered_pricing)
CREATE TABLE IF NOT EXISTS billing_tiered_prices (
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
CREATE INDEX IF NOT EXISTS idx_billing_tiered_prices_model ON billing_tiered_prices(model);

-- 11. billing_exchange_rates  (replaces: exchange_rates)
CREATE TABLE IF NOT EXISTS billing_exchange_rates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_currency TEXT NOT NULL,
    to_currency TEXT NOT NULL,
    rate BIGINT NOT NULL,
    updated_at INTEGER,
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
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
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
    total_length INTEGER DEFAULT 0,
    completed_length INTEGER DEFAULT 0,
    download_speed INTEGER DEFAULT 0,
    download_dir TEXT,
    filename TEXT,
    connections INTEGER DEFAULT 16,
    split INTEGER DEFAULT 5,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
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
    installed_at TEXT,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    error_message TEXT
);
CREATE INDEX IF NOT EXISTS idx_sys_installations_status ON sys_installations(status);
CREATE INDEX IF NOT EXISTS idx_sys_installations_installed_at ON sys_installations(installed_at)
