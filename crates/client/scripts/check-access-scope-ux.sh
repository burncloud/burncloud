#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require() {
  local needle="$1"
  local file="$2"
  if ! grep -Fq "$needle" "$file"; then
    echo "Missing access/scope UX contract: '$needle' in $file" >&2
    exit 1
  fi
}

forbid() {
  local needle="$1"
  local file="$2"
  if grep -Fq "$needle" "$file"; then
    echo "Access/scope regression: '$needle' in $file" >&2
    exit 1
  fi
}

# Keep client role interpretation aligned with the server's current persisted role model.
require 'eq_ignore_ascii_case("admin")' src/role_access.rs
forbid '"root" | "admin"' src/role_access.rs
forbid '"administrator"' src/role_access.rs
forbid '"operator"' src/role_access.rs
forbid '"owner"' src/role_access.rs

# Authentication chooses the workspace from returned roles instead of assuming every account is an operator.
require 'is_staff_roles(&response.roles)' src/critical_pages/auth.rs
require 'nav.replace(Route::Overview {})' src/critical_pages/auth.rs
require 'nav.replace(Route::Billing {})' src/critical_pages/auth.rs
require 'workspace shown after registration is determined by the roles returned' src/critical_pages/auth.rs

# AuthGate contains customer navigation at the UI layer. This is UX containment, not server authorization.
require 'CustomerConsoleLayout' src/auth_gate.rs
require 'is_staff_roles(&value.roles)' src/auth_gate.rs
require 'let customer_allowed = matches!(current, Route::Billing {})' src/auth_gate.rs
require 'navigator.replace(Route::Billing {})' src/auth_gate.rs
require 'This session does not have an operator role.' src/auth_gate.rs

# Customer shell intentionally exposes only account-scoped billing plus sign-out/public navigation.
require 'Billing & Usage' src/customer_layout.rs
require 'Customer access currently exposes only account-scoped billing.' src/customer_layout.rs
require 'Provider inventory, global logs, customer administration, API-key administration, guardrails, and system settings remain operator-only' src/customer_layout.rs
forbid 'Route::Providers' src/customer_layout.rs
forbid 'Route::Logs' src/customer_layout.rs
forbid 'Route::Customers' src/customer_layout.rs
forbid 'Route::APIKeys' src/customer_layout.rs
forbid 'Route::Guardrails' src/customer_layout.rs
forbid 'Route::Team' src/customer_layout.rs
forbid 'Route::Settings' src/customer_layout.rs

# Billing must never imply company/environment-wide financial scope when using the user-scoped public API.
require 'Billing & Usage' src/functional_pages/analytics.rs
require 'signed-in BurnCloud account' src/functional_pages/analytics.rs
require 'Account scope:' src/functional_pages/analytics.rs
require 'These numbers are not presented as company-wide or environment-wide spend.' src/functional_pages/analytics.rs
require 'Account Spend' src/functional_pages/analytics.rs
require 'Account Requests' src/functional_pages/analytics.rs
require 'Account Tokens' src/functional_pages/analytics.rs

# Overview explicitly separates account billing/usage from environment-wide operational data.
require 'Your usage' src/critical_pages/overview_live.rs
require 'billing scope: {username}' src/critical_pages/overview_live.rs
require 'Environment health' src/critical_pages/overview_live.rs
require 'environment-wide router log' src/critical_pages/overview_live.rs
require 'Unknown values stay unknown instead of being displayed as zero' src/critical_pages/overview_live.rs

# Performance may describe observed upstream diversity but may not infer configured failover from the sample.
require 'Observed upstream diversity describes this sample only' src/functional_pages/analytics_full.rs
require 'Single upstream observed' src/functional_pages/analytics_full.rs
require 'That does not prove they lack configured failover.' src/functional_pages/analytics_full.rs
require 'Review configured redundancy' src/functional_pages/analytics_full.rs
forbid 'Needs backup' src/functional_pages/analytics_full.rs

# New role-aware modules must stay in the runtime graph.
require 'pub mod customer_layout;' src/lib.rs
require 'pub mod role_access;' src/lib.rs

echo "Access and data-scope UX contracts OK"
