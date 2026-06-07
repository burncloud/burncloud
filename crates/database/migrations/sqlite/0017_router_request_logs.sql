-- Migration 0017: Router request logs table (SQLite)
-- Stores detailed request/response data for debugging and auditing.
-- Linked to router_logs via request_id foreign key.

-- SQLite doesn't have JSONB, use TEXT for JSON fields
-- SQLite doesn't support BIGSERIAL, use INTEGER PRIMARY KEY AUTOINCREMENT

CREATE TABLE IF NOT EXISTS router_request_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id TEXT NOT NULL UNIQUE,

    -- Request information (sanitized)
    request_body TEXT,                     -- JSON string (may be truncated)
    request_body_truncated INTEGER DEFAULT 0,  -- Boolean as INTEGER (0=false, 1=true)
    request_headers TEXT,                  -- JSON string (sanitized headers)

    -- Response information
    response_body TEXT,                    -- JSON string (may be truncated)
    response_body_truncated INTEGER DEFAULT 0,
    response_status INTEGER,

    -- Streaming response summary
    stream_chunk_count INTEGER DEFAULT 0,
    stream_first_chunk_latency_ms INTEGER,
    stream_last_chunk_latency_ms INTEGER,

    -- Routing decision information
    candidates TEXT,                       -- JSON array string
    candidates_count INTEGER DEFAULT 0,
    affinity_key TEXT,
    affinity_hit_channel_id INTEGER,
    failover_history TEXT,                 -- JSON array string

    -- Storage policy
    storage_policy TEXT DEFAULT 'summary',

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (request_id) REFERENCES router_logs(request_id) ON DELETE CASCADE
);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_router_request_logs_created_at ON router_request_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_router_request_logs_request_id ON router_request_logs(request_id);
CREATE INDEX IF NOT EXISTS idx_router_request_logs_storage_policy ON router_request_logs(storage_policy);