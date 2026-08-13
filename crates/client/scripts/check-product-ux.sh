#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail() {
  local file="$1"
  local message="$2"
  echo "::error file=$file::$message" >&2
  echo "$message" >&2
  exit 1
}

require() {
  local needle="$1"
  local file="$2"
  if ! grep -Fq "$needle" "$file"; then
    fail "$file" "Missing product UX contract: '$needle'"
  fi
}

forbid() {
  local needle="$1"
  local file="$2"
  if grep -Fq "$needle" "$file"; then
    fail "$file" "Prototype/backend-detail UI reintroduced: '$needle'"
  fi
}

line_of() {
  local needle="$1"
  local file="$2"
  grep -nF "$needle" "$file" | head -n1 | cut -d: -f1
}

# Operator navigation follows the real setup/observe workflow.
providers_line=$(line_of 'NavItem { to:Route::Providers' src/functional_layout.rs)
models_line=$(line_of 'NavItem { to:Route::Models' src/functional_layout.rs)
routes_line=$(line_of 'NavItem { to:Route::Routes' src/functional_layout.rs)
playground_line=$(line_of 'NavItem { to:Route::Playground' src/functional_layout.rs)
logs_line=$(line_of 'NavItem { to:Route::Logs' src/functional_layout.rs)
performance_line=$(line_of 'NavItem { to:Route::Evaluation' src/functional_layout.rs)
billing_line=$(line_of 'NavItem { to:Route::Billing' src/functional_layout.rs)
if ! (( providers_line < models_line && models_line < routes_line && routes_line < playground_line )); then
  fail src/functional_layout.rs "Traffic Setup must remain Providers -> Models -> Routes -> Playground"
fi
if ! (( logs_line < performance_line && performance_line < billing_line )); then
  fail src/functional_layout.rs "Monitoring must remain Logs -> Performance -> Billing"
fi
require 'label:"Performance"' src/functional_layout.rs
require 'Server Configured' src/functional_layout.rs
forbid 'Server Connected' src/functional_layout.rs
forbid 'Open request logs' src/functional_layout.rs
forbid 'System settings' src/functional_layout.rs

# Overview keeps unknown values unknown and separates account usage from environment health.
require 'Checking this environment' src/critical_pages/overview_live.rs
require 'Unknown values stay unknown instead of being displayed as zero' src/critical_pages/overview_live.rs
require 'Your usage' src/critical_pages/overview_live.rs
require 'Your Requests' src/critical_pages/overview_live.rs
require 'Your Tokens' src/critical_pages/overview_live.rs
require 'Your Spend' src/critical_pages/overview_live.rs
require 'Environment health' src/critical_pages/overview_live.rs
require 'Recent successful request observed' src/critical_pages/overview_live.rs
require 'Traffic path verified' src/critical_pages/overview_live.rs
forbid 'System Overview' src/critical_pages/overview_live.rs
forbid 'BurnCloud is ready to serve traffic' src/critical_pages/overview_live.rs

# Authentication exposes real account behavior only and routes by returned roles.
require 'Password recovery' src/critical_pages/auth.rs
require 'Account email' src/critical_pages/auth.rs
require 'workspace shown after registration is determined by the roles returned' src/critical_pages/auth.rs
require 'is_staff_roles(&response.roles)' src/critical_pages/auth.rs
require 'Route::Billing' src/critical_pages/auth.rs
forbid 'JWT' src/critical_pages/auth.rs
forbid 'href: "#privacy"' src/critical_pages/auth.rs
forbid 'href: "#terms"' src/critical_pages/auth.rs
forbid 'Onboarding Account Preference' src/critical_pages/auth.rs
forbid 'TierButton' src/critical_pages/auth.rs

# Public product surface does not revive historical attestation or invented benchmark claims.
require 'public_pages::{Home, Landing}' src/app.rs
require 'One API for your' src/public_pages.rs
require 'OpenAI-compatible endpoint' src/public_pages.rs
require 'It does not present ordinary routing metadata as cryptographic attestation.' src/public_pages.rs
forbid '100% Cryptographically Traceable' src/public_pages.rs
forbid 'Silicon-Attested' src/public_pages.rs
forbid 'TPM SIGNED' src/public_pages.rs
forbid '99.999%' src/public_pages.rs
forbid '12.8M' src/public_pages.rs

# Providers show product concepts and make intentional reactivation explicit.
require 'PROVIDER_TYPES' src/functional_pages/providers.rs
require 'Provider type' src/functional_pages/providers.rs
require 'Advanced routing & capacity' src/functional_pages/providers.rs
require 'Leave blank to keep stored credential' src/functional_pages/providers.rs
require 'Active Providers' src/functional_pages/providers.rs
require 'Routing Groups' src/functional_pages/providers.rs
require 'Repair Provider' src/functional_pages/providers.rs
require 'Saving this repair will reactivate routing' src/functional_pages/providers.rs
require 'Save & Reactivate' src/functional_pages/providers.rs
forbid 'Provider Type ID' src/functional_pages/providers.rs
forbid 'P{} • W{}' src/functional_pages/providers.rs
forbid '∞ RPM' src/functional_pages/providers.rs

# Models/Routes communicate configured availability rather than raw backend numbers.
require 'Needs backup' src/functional_pages/catalog.rs
require 'Protected' src/functional_pages/catalog.rs
require 'Unavailable' src/functional_pages/catalog.rs
require 'No active provider' src/functional_pages/catalog.rs
require 'Routing Policy' src/functional_pages/catalog.rs

# Playground requires explicit model and credential attribution.
require 'Playground is not ready yet' src/functional_pages/playground_live.rs
require 'Connect an active provider first' src/functional_pages/playground_live.rs
require 'Create an API key for the test' src/functional_pages/playground_live.rs
require 'Select a configured model' src/functional_pages/playground_live.rs
require 'Charge test to / API key' src/functional_pages/playground_live.rs
require 'Choose which account / API key should own this test.' src/functional_pages/playground_live.rs
require 'Playground will not silently pick the first active credential.' src/functional_pages/playground_live.rs
forbid 'first_active_api_token' src/functional_pages/playground_live.rs

# Customer administration exposes the server's real monetary defaults and blocks disabled-account funding.
require 'Manage customer accounts' src/critical_pages/customers_account.rs
require 'parse_currency_amount_nano' src/critical_pages/customers_account.rs
require 'step: "0.01"' src/critical_pages/customers_account.rs
require 'Current account defaults:' src/critical_pages/customers_account.rs
require 'Create Active Customer + $10 Credit' src/critical_pages/customers_account.rs
require 'Review wallet change' src/critical_pages/customers_account.rs
require 'Expected balance after' src/critical_pages/customers_account.rs
require 'Server-confirmed {selected_currency} balance' src/critical_pages/customers_account.rs
require 'Disabled accounts are visible for access review, but BurnCloud does not fund them from this page.' src/critical_pages/customers_account.rs
forbid 'Suspend Account' src/critical_pages/customers_account.rs

# Team is a truthful admin inventory, not fabricated staff CRUD.
require 'role_access::is_staff_role' src/functional_pages/team_live.rs
require 'Admin directory' src/functional_pages/team_live.rs
require 'Current session appears in the admin directory' src/functional_pages/team_live.rs
require 'current persisted role model exposes admin and user roles' src/functional_pages/team_live.rs
require 'This page is an access inventory, not a fake staff-management screen.' src/functional_pages/team_live.rs
forbid 'Invite Organization Member' src/functional_pages/team_live.rs
forbid 'Send Secure Invitation' src/functional_pages/team_live.rs

# API keys fail closed on ownership, present quota as USD spend, validate network rules, and protect one-time secrets.
require 'Spend quota semantics:' src/functional_pages/api_keys_live.rs
require 'parse_spend_limit_usd' src/functional_pages/api_keys_live.rs
require 'USD spend limit (optional)' src/functional_pages/api_keys_live.rs
require 'Stored as nano-USD (1 USD = 1,000,000,000 quota units).' src/functional_pages/api_keys_live.rs
require 'BurnCloud will not fall back to free-form ownership' src/functional_pages/api_keys_live.rs
require 'Disabled accounts are intentionally excluded from new credential ownership.' src/functional_pages/api_keys_live.rs
require 'active key(s) belong to an inactive or missing account' src/functional_pages/api_keys_live.rs
require 'Manage API Key' src/functional_pages/api_keys_live.rs
require 'Rotate API Key' src/functional_pages/api_keys_live.rs
require 'Delete API Key' src/functional_pages/api_keys_live.rs
require 'One-time credential reveal' src/functional_pages/api_keys_live.rs
require 'I saved this credential' src/functional_pages/api_keys_live.rs
require 'CIDR ranges such as 10.0.0.0/8 are not supported' src/functional_pages/api_keys_live.rs
require 'I understand this broadens network access for the credential.' src/functional_pages/api_keys_live.rs
forbid '.parse::<i64>().ok()' src/functional_pages/api_keys_live.rs
forbid 'Owner user ID' src/functional_pages/api_keys_live.rs

# Logs distinguish errors/timeouts/fallbacks/unknown and state the bounded sample scope.
require 'Problems (error + timeout)' src/functional_pages/logs_full.rs
require 'Show problems' src/functional_pages/logs_full.rs
require 'Show unknown' src/functional_pages/logs_full.rs
require 'up to the latest 200 environment-wide router-log records' src/functional_pages/logs_full.rs
require 'Loaded Cost is the cost stored on those rows, not an account bill or all-time environment spend.' src/functional_pages/logs_full.rs
require 'remain unavailable rather than falling back to zero' src/functional_pages/logs_full.rs
require '(200..400).contains(&self.status_code)' src/observability.rs
require 'eq_ignore_ascii_case("timeout")' src/observability.rs
require '"Unknown"' src/observability.rs
forbid 'self.status_code >= 500 ||' src/observability.rs

# Guardrails describe HTTP-derived risk signals truthfully.
require 'Request Health' src/functional_pages/guardrails_live.rs
require 'HTTP Error Events' src/functional_pages/guardrails_live.rs
require 'Affected IDs / Upstreams' src/functional_pages/guardrails_live.rs
require 'operational traffic indicators, not a threat-intelligence feed' src/functional_pages/guardrails_live.rs
require 'HTTP risk signals' src/functional_pages/guardrails_live.rs
require 'A 4xx does not automatically mean a malicious client' src/functional_pages/guardrails_live.rs
require 'DANGER ZONE' src/functional_pages/guardrails_live.rs
forbid 'Security Score' src/functional_pages/guardrails_live.rs
forbid 'Threat Sources' src/functional_pages/guardrails_live.rs

# Performance and Billing state their sample/account scopes.
require 'Performance' src/functional_pages/analytics_full.rs
require 'Sample observations' src/functional_pages/analytics_full.rs
require 'Single upstream observed' src/functional_pages/analytics_full.rs
require 'That does not prove they lack configured failover.' src/functional_pages/analytics_full.rs
require 'Billing & Usage' src/functional_pages/analytics.rs
require 'Account scope:' src/functional_pages/analytics.rs
require 'These numbers are not presented as company-wide or environment-wide spend.' src/functional_pages/analytics.rs
require 'Avg / Request' src/functional_pages/analytics.rs

# Settings only exposes verifiable runtime/cache operations.
require 'REACHABLE' src/functional_pages/settings.rs
require 'UNVERIFIED' src/functional_pages/settings.rs
require 'Retry runtime check' src/functional_pages/settings.rs
require 'let cache_operational = cache_enabled == Some(true) && cache_connected == Some(true);' src/functional_pages/settings.rs
require "Clear Application Cache deletes BurnCloud's bc:* Redis cache keys." src/functional_pages/settings.rs
require 'MAINTENANCE' src/functional_pages/settings.rs
forbid '"CONNECTED"' src/functional_pages/settings.rs

echo "Product UX contracts OK"
