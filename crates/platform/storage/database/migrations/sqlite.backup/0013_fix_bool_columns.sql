-- Fix SQLite bool columns: BOOLEAN → INTEGER
-- The sqlx Any driver cannot decode SQLite BOOLEAN into Rust bool.
-- These columns store 0/1 integers anyway, so just change the declared type.

-- channel_protocol_configs.is_default
CREATE TABLE channel_protocol_configs_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_type INTEGER NOT NULL,
    api_version TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    chat_endpoint TEXT,
    embed_endpoint TEXT,
    models_endpoint TEXT,
    request_mapping TEXT,
    response_mapping TEXT,
    detection_rules TEXT,
    created_at INTEGER,
    updated_at INTEGER
);

INSERT INTO channel_protocol_configs_new SELECT * FROM channel_protocol_configs;
DROP TABLE channel_protocol_configs;
ALTER TABLE channel_protocol_configs_new RENAME TO channel_protocol_configs;

-- channel_abilities.enabled
CREATE TABLE channel_abilities_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_name TEXT NOT NULL,
    model TEXT NOT NULL,
    channel_id INTEGER NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    weight INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER,
    updated_at INTEGER,
    FOREIGN KEY (channel_id) REFERENCES channels(id)
);

INSERT INTO channel_abilities_new SELECT * FROM channel_abilities;
DROP TABLE channel_abilities;
ALTER TABLE channel_abilities_new RENAME TO channel_abilities;
