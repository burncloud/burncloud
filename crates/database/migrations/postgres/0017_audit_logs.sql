-- Audit logging for sensitive operations (Issue #248)
-- Records all sensitive operations for compliance and security auditing

-- Audit logs table
-- Note: This table is append-only. UPDATE and DELETE operations should be prevented at application level.
CREATE TABLE IF NOT EXISTS audit_logs (
    id SERIAL PRIMARY KEY,
    timestamp BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT,
    actor_id TEXT NOT NULL,                  -- User ID who performed the action
    actor_ip TEXT,                           -- IP address of the actor
    action TEXT NOT NULL,                    -- Action type (e.g., 'user.login', 'channel.create', 'token.delete')
    resource_type TEXT,                      -- Type of resource affected (e.g., 'user', 'channel', 'token')
    resource_id TEXT,                        -- ID of the resource affected
    status TEXT NOT NULL DEFAULT 'success',  -- 'success' or 'failure'
    changes TEXT,                            -- JSON string containing before/after values
    metadata TEXT                            -- Additional metadata as JSON
);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_actor_id ON audit_logs(actor_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_type ON audit_logs(resource_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_status ON audit_logs(status);
