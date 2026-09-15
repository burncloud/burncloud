-- Migration 0011: MVP scheduler layering — three-table column additions (SQLite)
-- See docs/design/channel-scheduler-hqos.md § 阶段 MVP for the full rationale.
-- Audit decisions D12 / E-D9: schema changes for MVP + 阶段 2 + 阶段 3-observability
-- bundled into a single migration to avoid drift across phases.
--
-- Note: SQLite does not support IF NOT EXISTS on ALTER TABLE, so the migration
-- runner swallows "duplicate column" errors which makes re-applying on existing
-- databases a no-op. The runner splits on statement terminators, so do not
-- place that punctuation inside comments.

-- 1. router_tokens — per-token Order Type + price cap (L1 Classifier inputs)
ALTER TABLE router_tokens ADD COLUMN order_type VARCHAR(16) DEFAULT 'value';
ALTER TABLE router_tokens ADD COLUMN price_cap_nanodollars BIGINT;

-- 2. channel_providers — RPM/TPM hard caps + three-color reservation policy
--    (L2 Shaper inputs, consumed by rate_budget::InMemoryBudget::configure).
ALTER TABLE channel_providers ADD COLUMN rpm_cap INTEGER;
ALTER TABLE channel_providers ADD COLUMN tpm_cap BIGINT;
ALTER TABLE channel_providers ADD COLUMN reservation_green REAL DEFAULT 0.4;
ALTER TABLE channel_providers ADD COLUMN reservation_yellow REAL DEFAULT 0.4;
ALTER TABLE channel_providers ADD COLUMN reservation_red REAL DEFAULT 0.2;

-- 3. router_logs — L6 Observability fields (which layer made the decision,
--    what color was attached). Required for affinity_hit / shaper_reject
--    / scorer_picked / failover_N reporting in Grafana.
ALTER TABLE router_logs ADD COLUMN layer_decision VARCHAR(32);
ALTER TABLE router_logs ADD COLUMN traffic_color CHAR(1);
