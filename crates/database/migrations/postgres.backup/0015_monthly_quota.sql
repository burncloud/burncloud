-- Monthly quota billing system (Issue #232)
-- Supports two billing modes: per_request and per_token

-- Billing plans table
CREATE TABLE IF NOT EXISTS billing_plans (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    monthly_fee BIGINT NOT NULL,            -- CNY in nanodollars (9 decimal precision)
    billing_mode TEXT NOT NULL,             -- 'per_request' or 'per_token'
    request_limit BIGINT,                   -- For per_request mode
    token_limit BIGINT,                     -- For per_token mode
    channel_id INTEGER NOT NULL,            -- Bind to specific upstream channel
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT,
    updated_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT,
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_billing_plans_channel_id ON billing_plans(channel_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_billing_plans_name ON billing_plans(name);

-- Billing subscriptions table
CREATE TABLE IF NOT EXISTS billing_subscriptions (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    plan_id INTEGER NOT NULL,
    channel_id INTEGER NOT NULL,            -- Inherited from plan
    status TEXT NOT NULL DEFAULT 'active',  -- 'active', 'expired', 'cancelled'
    quota_used BIGINT NOT NULL DEFAULT 0,   -- Used quota (requests or tokens)
    quota_limit BIGINT NOT NULL,            -- Quota limit (from plan)
    expires_at BIGINT NOT NULL,             -- Expiration timestamp (ms)
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT,
    updated_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (plan_id) REFERENCES billing_plans(id) ON DELETE RESTRICT,
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_billing_subscriptions_user_id ON billing_subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_billing_subscriptions_plan_id ON billing_subscriptions(plan_id);
CREATE INDEX IF NOT EXISTS idx_billing_subscriptions_channel_id ON billing_subscriptions(channel_id);
CREATE INDEX IF NOT EXISTS idx_billing_subscriptions_status ON billing_subscriptions(status);
CREATE INDEX IF NOT EXISTS idx_billing_subscriptions_expires_at ON billing_subscriptions(expires_at);
