-- Migration 0017: Router request logs table (PostgreSQL)
-- Stores detailed request/response data for debugging and auditing.
-- Linked to router_logs via request_id foreign key.

CREATE TABLE IF NOT EXISTS router_request_logs (
    id BIGSERIAL PRIMARY KEY,
    request_id TEXT NOT NULL UNIQUE,

    -- Request information (sanitized)
    request_body JSONB,                    -- Request body (may be truncated)
    request_body_truncated BOOLEAN DEFAULT FALSE,
    request_headers JSONB,                 -- Sanitized headers (sensitive fields redacted)

    -- Response information
    response_body JSONB,                   -- Response body (may be truncated)
    response_body_truncated BOOLEAN DEFAULT FALSE,
    response_status INTEGER,               -- HTTP status code (redundant with router_logs but useful for quick lookup)

    -- Streaming response summary (for stream requests, we don't store full response)
    stream_chunk_count INTEGER DEFAULT 0,  -- Number of SSE chunks received
    stream_first_chunk_latency_ms BIGINT,  -- Time to first chunk
    stream_last_chunk_latency_ms BIGINT,   -- Time to last chunk

    -- Routing decision information
    candidates JSONB,                      -- List of candidate channels considered
    candidates_count INTEGER DEFAULT 0,    -- Number of candidates
    affinity_key TEXT,                     -- Affinity cache key (session_id + model)
    affinity_hit_channel_id INTEGER,       -- Channel ID from affinity cache hit (if any)
    failover_history JSONB,                -- Array of failover attempts with errors

    -- Storage policy (controls what gets recorded)
    -- 'full': complete request/response (dev/debug)
    -- 'summary': metadata only, no body (production default)
    -- 'none': skip recording (high traffic)
    storage_policy VARCHAR(16) DEFAULT 'summary',

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (request_id) REFERENCES router_logs(request_id) ON DELETE CASCADE
);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_router_request_logs_created_at ON router_request_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_router_request_logs_request_id ON router_request_logs(request_id);
CREATE INDEX IF NOT EXISTS idx_router_request_logs_storage_policy ON router_request_logs(storage_policy);

-- Comment on table for documentation
COMMENT ON TABLE router_request_logs IS 'Detailed request/response logs for debugging. Linked to router_logs via request_id.';
COMMENT ON COLUMN router_request_logs.storage_policy IS 'Controls verbosity: full (complete bodies), summary (metadata only), none (skip)';
COMMENT ON COLUMN router_request_logs.candidates IS 'JSON array of candidate channels: [{id, name, protocol, priority, ...}]';
COMMENT ON COLUMN router_request_logs.failover_history IS 'JSON array of failover attempts: [{attempt, channel_id, error, latency_ms}]';