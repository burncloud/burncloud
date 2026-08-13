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

# Customer administration exposes server-side money/account defaults instead of hiding them.
require 'role_access::{is_staff_role, is_staff_roles}' src/critical_pages/customers_live.rs
require 'the server creates new users as Active and currently grants a $10.00 USD signup wallet credit' src/critical_pages/customers_live.rs
require 'Create Active Customer + $10 Credit' src/critical_pages/customers_live.rs
require 'Role is verified from the server response after creation.' src/critical_pages/customers_live.rs
require 'Disabled accounts are visible for access review, but BurnCloud does not fund them from this page.' src/critical_pages/customers_live.rs
require 'disabled: user.status != 1' src/critical_pages/customers_live.rs
require 'Server-confirmed {selected_currency} balance' src/critical_pages/customers_live.rs
require 'pub use customers_live::Customers;' src/critical_pages.rs
forbid 'mod customers_portable;' src/critical_pages.rs
forbid 'pub use customers_portable::Customers;' src/critical_pages.rs

# API-key ownership must fail closed and cost quota must be presented with its actual USD meaning.
require 'let owner_directory_ready = user_snapshot.as_ref().is_some_and(Result::is_ok);' src/functional_pages/api_keys_live.rs
require 'let active_users: Vec<User>' src/functional_pages/api_keys_live.rs
require '.filter(|user| user.status == 1)' src/functional_pages/api_keys_live.rs
require 'let create_ready = owner_directory_ready && !active_users.is_empty();' src/functional_pages/api_keys_live.rs
require 'BurnCloud will not fall back to free-form ownership' src/functional_pages/api_keys_live.rs
require 'Choose an active owner returned by the current account directory.' src/functional_pages/api_keys_live.rs
require 'router credential quota is charged from calculated request cost in nano-USD' src/functional_pages/api_keys_live.rs
require 'Spend Used / Limit' src/functional_pages/api_keys_live.rs
require 'active key(s) belong to an inactive or missing account' src/functional_pages/api_keys_live.rs
require 'server-side enforcement is tracked separately' src/functional_pages/api_keys_live.rs
forbid 'Owner user ID' src/functional_pages/api_keys_live.rs

# Performance may describe observed upstream diversity but may not infer configured failover from the sample.
require 'Observed upstream diversity describes this sample only' src/functional_pages/analytics_full.rs
require 'Single upstream observed' src/functional_pages/analytics_full.rs
require 'That does not prove they lack configured failover.' src/functional_pages/analytics_full.rs
require 'Review configured redundancy' src/functional_pages/analytics_full.rs
forbid 'Needs backup' src/functional_pages/analytics_full.rs

# New role-aware modules must stay in the runtime graph.
require 'pub mod customer_layout;' src/lib.rs
require 'pub mod role_access;' src/lib.rs
require 'pub use api_keys_live::APIKeys;' src/functional_pages/mod.rs

# Historical combined access module stays outside the active graph so stale Team/API-key logic cannot be mistaken for runtime truth.
forbid 'mod access_live;' src/functional_pages/mod.rs
forbid 'pub use access_live::APIKeys;' src/functional_pages/mod.rs

echo "Access and data-scope UX contracts OK"
