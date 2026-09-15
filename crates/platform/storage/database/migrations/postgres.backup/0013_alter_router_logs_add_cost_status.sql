-- Migration 0013: Billing observability — cost_status column on router_logs (PostgreSQL)
-- Tracks whether cost calculation succeeded or why it returned 0.
-- Values: "ok", "price_missing", "calc_error", "no_model", or NULL (pre-migration rows).

ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS cost_status TEXT DEFAULT NULL;
