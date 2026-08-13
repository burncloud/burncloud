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

# Current persisted role model: admin is the operator role; other accounts use customer scope.
require 'eq_ignore_ascii_case("admin")' src/role_access.rs
forbid '"administrator"' src/role_access.rs
forbid '"operator"' src/role_access.rs
forbid '"owner"' src/role_access.rs
require 'is_staff_roles(&response.roles)' src/critical_pages/auth.rs
require 'nav.replace(Route::Overview {})' src/critical_pages/auth.rs
require 'nav.replace(Route::Billing {})' src/critical_pages/auth.rs
require 'CustomerConsoleLayout' src/auth_gate.rs
require 'let customer_allowed = matches!(current, Route::Billing {})' src/auth_gate.rs
require 'navigator.replace(Route::Billing {})' src/auth_gate.rs

# Customer shell exposes only account-scoped billing in the product UI.
require 'Billing & Usage' src/customer_layout.rs
require 'Customer access currently exposes only account-scoped billing.' src/customer_layout.rs
forbid 'Route::Providers' src/customer_layout.rs
forbid 'Route::Logs' src/customer_layout.rs
forbid 'Route::Customers' src/customer_layout.rs
forbid 'Route::APIKeys' src/customer_layout.rs
forbid 'Route::Guardrails' src/customer_layout.rs
forbid 'Route::Team' src/customer_layout.rs
forbid 'Route::Settings' src/customer_layout.rs

# Billing and Overview keep account and environment scopes separate.
require 'These numbers are not presented as company-wide or environment-wide spend.' src/functional_pages/analytics.rs
require 'Account Spend' src/functional_pages/analytics.rs
require 'Account Requests' src/functional_pages/analytics.rs
require 'Account Tokens' src/functional_pages/analytics.rs
require 'Your usage' src/critical_pages/overview_live.rs
require 'Environment health' src/critical_pages/overview_live.rs
require 'Unknown values stay unknown instead of being displayed as zero' src/critical_pages/overview_live.rs

# Customer administration exposes real server defaults and blocks funding disabled accounts.
require 'role_access::{is_staff_role, is_staff_roles}' src/critical_pages/customers_account.rs
require 'the server creates new users as Active and currently grants a $10.00 USD signup wallet credit' src/critical_pages/customers_account.rs
require 'Create Active Customer + $10 Credit' src/critical_pages/customers_account.rs
require 'Role is verified from the server response after creation.' src/critical_pages/customers_account.rs
require 'Disabled accounts are visible for access review, but BurnCloud does not fund them from this page.' src/critical_pages/customers_account.rs
require 'disabled: user.status != 1' src/critical_pages/customers_account.rs
require 'Server-confirmed {selected_currency} balance' src/critical_pages/customers_account.rs
require 'pub use customers_account::Customers;' src/critical_pages.rs
forbid 'mod customers_live;' src/critical_pages.rs
forbid 'mod customers_portable;' src/critical_pages.rs

# API-key ownership fails closed and quota is shown with its real USD cost meaning.
require 'let owner_directory_ready = user_snapshot.as_ref().is_some_and(Result::is_ok);' src/functional_pages/api_keys_live.rs
require 'let create_ready = owner_directory_ready && !active_users.is_empty();' src/functional_pages/api_keys_live.rs
require 'BurnCloud will not fall back to free-form ownership' src/functional_pages/api_keys_live.rs
require 'router credential quota is charged from calculated request cost in nano-USD' src/functional_pages/api_keys_live.rs
require 'Spend Used / Limit' src/functional_pages/api_keys_live.rs
require 'active key(s) belong to an inactive or missing account' src/functional_pages/api_keys_live.rs
forbid 'Owner user ID' src/functional_pages/api_keys_live.rs

# Cache maintenance is only available after enabled+connected Redis state is verified.
require 'let cache_operational = cache_enabled == Some(true) && cache_connected == Some(true);' src/functional_pages/settings.rs
require 'Application caching is disabled. There is no active Redis cache namespace to clear.' src/functional_pages/settings.rs
require "Clear Application Cache deletes BurnCloud's bc:* Redis cache keys." src/functional_pages/settings.rs
require 'disabled: busy() || !cache_operational || !confirm_clear()' src/functional_pages/settings.rs
require 'Cache state is not enabled and connected; maintenance was not sent.' src/functional_pages/settings.rs

# Logs are a bounded environment sample. Missing/non-final status is never inferred as success.
require 'up to the latest 200 environment-wide router-log records' src/functional_pages/logs_full.rs
require 'Loaded Cost is the cost stored on those rows, not an account bill or all-time environment spend.' src/functional_pages/logs_full.rs
require 'remain unavailable rather than falling back to zero' src/functional_pages/logs_full.rs
require 'Missing or non-final status codes stay unknown instead of being counted as success.' src/functional_pages/logs_full.rs
require 'option { value: "unknown", "Unknown" }' src/functional_pages/logs_full.rs
require '(200..400).contains(&self.status_code)' src/observability.rs
require '"Unknown"' src/observability.rs

# Performance sample diversity is not configured redundancy.
require 'Observed upstream diversity describes this sample only' src/functional_pages/analytics_full.rs
require 'Single upstream observed' src/functional_pages/analytics_full.rs
require 'That does not prove they lack configured failover.' src/functional_pages/analytics_full.rs

# Stale combined access implementation stays out of the runtime graph.
require 'pub use api_keys_live::APIKeys;' src/functional_pages/mod.rs
forbid 'mod access_live;' src/functional_pages/mod.rs
forbid 'pub use access_live::APIKeys;' src/functional_pages/mod.rs

echo "Access and data-scope UX contracts OK"
