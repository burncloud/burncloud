-- Migration 0009: Router configuration tables (PostgreSQL)
-- Idempotent: all statements use CREATE TABLE IF NOT EXISTS / CREATE INDEX IF NOT EXISTS

CREATE TABLE IF NOT EXISTS router_upstreams (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    api_key TEXT NOT NULL,
    match_path TEXT NOT NULL,
    auth_type TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    protocol TEXT NOT NULL DEFAULT 'openai',
    param_override TEXT,
    header_override TEXT,
    api_version TEXT
);

CREATE TABLE IF NOT EXISTS router_tokens (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    status TEXT NOT NULL,
    quota_limit BIGINT NOT NULL DEFAULT -1,
    used_quota BIGINT NOT NULL DEFAULT 0,
    expired_time BIGINT NOT NULL DEFAULT -1,
    accessed_time BIGINT NOT NULL DEFAULT 0,
    name TEXT
);

CREATE TABLE IF NOT EXISTS router_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    strategy TEXT NOT NULL DEFAULT 'round_robin',
    match_path TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS router_group_members (
    group_id TEXT NOT NULL,
    upstream_id TEXT NOT NULL,
    weight INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (group_id, upstream_id)
)
