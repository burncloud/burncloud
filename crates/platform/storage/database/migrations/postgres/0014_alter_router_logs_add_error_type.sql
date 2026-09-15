-- Migration 0014: Error classification — error_type column on router_logs (PostgreSQL)
-- Classifies why a request failed: "upstream_error", "timeout", "auth_failed",
-- "rate_limit", "router_reject", or NULL for successful requests.

ALTER TABLE router_logs ADD COLUMN IF NOT EXISTS error_type TEXT DEFAULT NULL;
