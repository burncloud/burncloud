# Database Migration Guide

This document describes the database schema changes and migration procedures for BurnCloud.

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 1.1.0 | 2025-02-15 | Billing, quota, and token management features |

## Migrating to v1.1.0

### Overview

This version introduces:
- **Pricing System**: New `prices` table for model cost calculation
- **Token Management**: New fields `accessed_time` and `expired_time` in tokens table
- **Billing Integration**: New `cost` field in router logs for request cost tracking

### Schema Changes

#### 1. New `prices` Table

Stores per-model pricing information for cost calculation.

```sql
-- SQLite
CREATE TABLE IF NOT EXISTS prices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model TEXT NOT NULL UNIQUE,
    input_price REAL NOT NULL DEFAULT 0,
    output_price REAL NOT NULL DEFAULT 0,
    currency TEXT DEFAULT 'USD',
    alias_for TEXT,
    created_at INTEGER,
    updated_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model);

-- PostgreSQL
CREATE TABLE IF NOT EXISTS prices (
    id SERIAL PRIMARY KEY,
    model VARCHAR(255) NOT NULL UNIQUE,
    input_price DOUBLE PRECISION NOT NULL DEFAULT 0,
    output_price DOUBLE PRECISION NOT NULL DEFAULT 0,
    currency VARCHAR(10) DEFAULT 'USD',
    alias_for VARCHAR(255),
    created_at BIGINT,
    updated_at BIGINT
);
CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model);
```

**Fields:**
- `model`: Model identifier (e.g., `gpt-4`, `claude-3-opus`)
- `input_price`: Cost per 1 million prompt tokens
- `output_price`: Cost per 1 million completion tokens
- `currency`: Currency code (default: `USD`)
- `alias_for`: If set, this model uses another model's pricing

#### 2. Token Table Updates

New fields added to the `tokens` table for expiry and access tracking:

```sql
-- Add accessed_time column (SQLite)
ALTER TABLE tokens ADD COLUMN accessed_time INTEGER;

-- Add expired_time column (SQLite)
ALTER TABLE tokens ADD COLUMN expired_time INTEGER DEFAULT -1;
```

```sql
-- PostgreSQL
ALTER TABLE tokens ADD COLUMN accessed_time BIGINT;
ALTER TABLE tokens ADD COLUMN expired_time BIGINT DEFAULT -1;
```

**Fields:**
- `accessed_time`: Unix timestamp of last token usage
- `expired_time`: Token expiration time
  - `-1`: Never expires (default)
  - `> 0`: Unix timestamp of expiration

#### 3. Router Logs Cost Field

New field for tracking request costs:

```sql
-- SQLite
ALTER TABLE router_logs ADD COLUMN cost REAL DEFAULT 0.0;

-- PostgreSQL
ALTER TABLE router_logs ADD COLUMN cost DOUBLE PRECISION DEFAULT 0.0;
```

### Migration Scripts

#### Fresh Installation

For new installations, the schema is automatically initialized when starting the server. No manual migration needed.

#### Upgrading from v1.0.x

Run the following SQL commands to upgrade your database:

**SQLite:**

```sql
-- 1. Create prices table
CREATE TABLE IF NOT EXISTS prices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model TEXT NOT NULL UNIQUE,
    input_price REAL NOT NULL DEFAULT 0,
    output_price REAL NOT NULL DEFAULT 0,
    currency TEXT DEFAULT 'USD',
    alias_for TEXT,
    created_at INTEGER,
    updated_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model);

-- 2. Add token fields (ignore if columns exist)
-- SQLite doesn't support IF NOT EXISTS for ALTER TABLE
-- These may fail if columns already exist - that's OK
ALTER TABLE tokens ADD COLUMN accessed_time INTEGER;
ALTER TABLE tokens ADD COLUMN expired_time INTEGER DEFAULT -1;

-- 3. Add cost field to router_logs
ALTER TABLE router_logs ADD COLUMN cost REAL DEFAULT 0.0;

-- 4. Insert default pricing data
INSERT OR IGNORE INTO prices (model, input_price, output_price, currency, created_at, updated_at)
VALUES
    -- OpenAI models
    ('gpt-4', 30.0, 60.0, 'USD', strftime('%s', 'now'), strftime('%s', 'now')),
    ('gpt-4-turbo', 10.0, 30.0, 'USD', strftime('%s', 'now'), strftime('%s', 'now')),
    ('gpt-4o', 2.5, 10.0, 'USD', strftime('%s', 'now'), strftime('%s', 'now')),
    ('gpt-4o-mini', 0.15, 0.60, 'USD', strftime('%s', 'now'), strftime('%s', 'now')),
    ('gpt-3.5-turbo', 0.50, 1.50, 'USD', strftime('%s', 'now'), strftime('%s', 'now')),
    -- Anthropic models
    ('claude-3-opus', 15.0, 75.0, 'USD', strftime('%s', 'now'), strftime('%s', 'now')),
    ('claude-3-sonnet', 3.0, 15.0, 'USD', strftime('%s', 'now'), strftime('%s', 'now')),
    ('claude-3-haiku', 0.25, 1.25, 'USD', strftime('%s', 'now'), strftime('%s', 'now')),
    ('claude-3-5-sonnet', 3.0, 15.0, 'USD', strftime('%s', 'now'), strftime('%s', 'now')),
    -- Google models
    ('gemini-1.5-pro', 3.5, 10.5, 'USD', strftime('%s', 'now'), strftime('%s', 'now')),
    ('gemini-1.5-flash', 0.075, 0.30, 'USD', strftime('%s', 'now'), strftime('%s', 'now')),
    ('gemini-pro', 0.50, 1.50, 'USD', strftime('%s', 'now'), strftime('%s', 'now'));

-- 5. Update existing tokens with default expiry (never expires)
UPDATE tokens SET expired_time = -1 WHERE expired_time IS NULL;
```

**PostgreSQL:**

```sql
-- 1. Create prices table
CREATE TABLE IF NOT EXISTS prices (
    id SERIAL PRIMARY KEY,
    model VARCHAR(255) NOT NULL UNIQUE,
    input_price DOUBLE PRECISION NOT NULL DEFAULT 0,
    output_price DOUBLE PRECISION NOT NULL DEFAULT 0,
    currency VARCHAR(10) DEFAULT 'USD',
    alias_for VARCHAR(255),
    created_at BIGINT,
    updated_at BIGINT
);
CREATE INDEX IF NOT EXISTS idx_prices_model ON prices(model);

-- 2. Add token fields
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS accessed_time BIGINT;
ALTER TABLE tokens ADD COLUMN IF NOT EXISTS expired_time BIGINT DEFAULT -1;

-- 3. Add cost field to router_logs
ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS cost DOUBLE PRECISION DEFAULT 0.0;

-- 4. Insert default pricing data
INSERT INTO prices (model, input_price, output_price, currency, created_at, updated_at)
VALUES
    ('gpt-4', 30.0, 60.0, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('gpt-4-turbo', 10.0, 30.0, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('gpt-4o', 2.5, 10.0, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('gpt-4o-mini', 0.15, 0.60, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('gpt-3.5-turbo', 0.50, 1.50, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('claude-3-opus', 15.0, 75.0, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('claude-3-sonnet', 3.0, 15.0, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('claude-3-haiku', 0.25, 1.25, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('claude-3-5-sonnet', 3.0, 15.0, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('gemini-1.5-pro', 3.5, 10.5, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('gemini-1.5-flash', 0.075, 0.30, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
    ('gemini-pro', 0.50, 1.50, 'USD', EXTRACT(EPOCH FROM NOW())::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT)
ON CONFLICT (model) DO NOTHING;

-- 5. Update existing tokens with default expiry
UPDATE tokens SET expired_time = -1 WHERE expired_time IS NULL;
```

### Post-Migration Verification

After running migrations, verify:

1. **Prices table exists**:
   ```sql
   SELECT COUNT(*) FROM prices;
   -- Should return 12 (or number of default prices)
   ```

2. **Token fields added**:
   ```sql
   SELECT accessed_time, expired_time FROM tokens LIMIT 1;
   ```

3. **Router logs cost field**:
   ```sql
   SELECT cost FROM router_logs LIMIT 1;
   ```

### CLI Commands for Pricing

After migration, manage pricing via CLI:

```bash
# List all prices
burncloud price list

# Set custom price
burncloud price set my-model --input 10.0 --output 20.0

# Create alias
burncloud price set gpt-4-32k --alias gpt-4

# Get specific price
burncloud price get gpt-4
```

### Rollback

To rollback the migration (not recommended):

**SQLite:**
```sql
DROP TABLE IF EXISTS prices;
-- SQLite doesn't support DROP COLUMN, would need to recreate table
```

**PostgreSQL:**
```sql
DROP TABLE IF EXISTS prices;
ALTER TABLE tokens DROP COLUMN IF EXISTS accessed_time;
ALTER TABLE tokens DROP COLUMN IF EXISTS expired_time;
ALTER TABLE router_logs DROP COLUMN IF EXISTS cost;
```

## Notes

- The migration scripts are idempotent where possible
- Default prices are based on published rates as of 2025-02-15
- Prices should be updated according to current provider pricing
- The `alias_for` feature allows model variants to share pricing
