#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require() {
  local needle="$1"
  local file="$2"
  if ! grep -Fq "$needle" "$file"; then
    echo "Missing product UX contract: '$needle' in $file" >&2
    exit 1
  fi
}

forbid() {
  local needle="$1"
  local file="$2"
  if grep -Fq "$needle" "$file"; then
    echo "Prototype/backend-detail UI reintroduced: '$needle' in $file" >&2
    exit 1
  fi
}

line_of() {
  local needle="$1"
  local file="$2"
  grep -nF "$needle" "$file" | head -n1 | cut -d: -f1
}

# Navigation follows the operator workflow: configure supply -> understand catalog/routing -> test traffic.
providers_line=$(line_of 'NavItem { to:Route::Providers' src/functional_layout.rs)
models_line=$(line_of 'NavItem { to:Route::Models' src/functional_layout.rs)
routes_line=$(line_of 'NavItem { to:Route::Routes' src/functional_layout.rs)
playground_line=$(line_of 'NavItem { to:Route::Playground' src/functional_layout.rs)
logs_line=$(line_of 'NavItem { to:Route::Logs' src/functional_layout.rs)
performance_line=$(line_of 'NavItem { to:Route::Evaluation' src/functional_layout.rs)
billing_line=$(line_of 'NavItem { to:Route::Billing' src/functional_layout.rs)

if ! (( providers_line < models_line && models_line < routes_line && routes_line < playground_line )); then
  echo "Traffic Setup navigation must remain Providers -> Models -> Routes -> Playground" >&2
  exit 1
fi
if ! (( logs_line < performance_line && performance_line < billing_line )); then
  echo "Monitoring navigation must remain Logs -> Performance -> Billing" >&2
  exit 1
fi
require 'label:"Performance"' src/functional_layout.rs

# Overview must distinguish configured prerequisites from a verified successful traffic path.
require 'Setup & verification' src/critical_pages/dashboard.rs
require 'Setup is complete — verify one real request' src/critical_pages/dashboard.rs
require 'Traffic path verified' src/critical_pages/dashboard.rs
require 'has_successful_request' src/critical_pages/dashboard.rs
require 'Run verification test' src/critical_pages/dashboard.rs
require 'router_status_label' src/critical_pages/dashboard.rs
require 'eq_ignore_ascii_case("timeout")' src/critical_pages/dashboard.rs
forbid 'BurnCloud is ready to serve traffic' src/critical_pages/dashboard.rs

# Auth exposes real account capabilities only, without developer implementation notes or fake legal acceptance.
forbid 'Onboarding Account Preference' src/critical_pages/auth.rs
forbid 'TierButton' src/critical_pages/auth.rs
forbid 'Company / Team' src/critical_pages/auth.rs
forbid 'auth-tabs' src/critical_pages/auth.rs
require 'Password recovery' src/critical_pages/auth.rs
require 'Account email' src/critical_pages/auth.rs
require 'Manage providers, traffic, customers, access, and billing' src/critical_pages/auth.rs
forbid 'BurnCloud stores the authenticated session locally' src/critical_pages/auth.rs
forbid 'JWT' src/critical_pages/auth.rs
forbid 'href: "#privacy"' src/critical_pages/auth.rs
forbid 'href: "#terms"' src/critical_pages/auth.rs
forbid 'let mut terms' src/critical_pages/auth.rs
forbid 'Terms of Service and Privacy Policy' src/critical_pages/auth.rs
forbid 'Registration now shows only fields' src/critical_pages/auth.rs

# Public product surfaces must describe current capabilities, not old prototype attestation/benchmark claims.
require 'public_pages::{Home, Landing}' src/app.rs
require 'pub mod public_pages;' src/lib.rs
forbid 'pub mod pages;' src/lib.rs
require 'One API for your' src/public_pages.rs
require 'OpenAI-compatible endpoint' src/public_pages.rs
require 'It does not present ordinary routing metadata as cryptographic attestation.' src/public_pages.rs
forbid '100% Cryptographically Traceable' src/public_pages.rs
forbid 'Silicon-Attested' src/public_pages.rs
forbid 'TPM SIGNED' src/public_pages.rs
forbid '99.999%' src/public_pages.rs
forbid '12.8M' src/public_pages.rs
forbid '$4,766' src/public_pages.rs
require 'BurnCloud - AI Gateway Console' src/app.rs

# Providers present product concepts instead of raw enum IDs/shorthand and protect destructive changes.
require 'PROVIDER_TYPES' src/functional_pages/providers.rs
require 'Provider type' src/functional_pages/providers.rs
require 'Advanced routing & capacity' src/functional_pages/providers.rs
require 'Leave blank to keep stored credential' src/functional_pages/providers.rs
require 'pending_delete' src/functional_pages/providers.rs
require 'Active Providers' src/functional_pages/providers.rs
require 'Routing Groups' src/functional_pages/providers.rs
require 'Priority {} • Weight {}' src/functional_pages/providers.rs
require 'No limits' src/functional_pages/providers.rs
forbid 'Provider Type ID' src/functional_pages/providers.rs
forbid 'P{} • W{}' src/functional_pages/providers.rs
forbid '∞ RPM' src/functional_pages/providers.rs

# Models/Routes communicate understandable service availability and resilience.
require 'Needs backup' src/functional_pages/catalog.rs
require 'Protected' src/functional_pages/catalog.rs
require 'Unavailable' src/functional_pages/catalog.rs
require 'No active provider' src/functional_pages/catalog.rs
require 'Routing Policy' src/functional_pages/catalog.rs

# Playground is a guided end-to-end test and never silently chooses customer/API-key attribution.
require 'Playground is not ready yet' src/functional_pages/playground_live.rs
require 'Connect an active provider first' src/functional_pages/playground_live.rs
require 'Create an API key for the test' src/functional_pages/playground_live.rs
require 'Select a configured model' src/functional_pages/playground_live.rs
require 'Charge test to / API key' src/functional_pages/playground_live.rs
require 'Choose which account / API key should own this test.' src/functional_pages/playground_live.rs
require 'Playground will not silently pick the first active credential.' src/functional_pages/playground_live.rs
require 'Send Test Request' src/functional_pages/playground_live.rs
forbid 'first_active_api_token' src/functional_pages/playground_live.rs

# Customers and staff have separate product responsibilities.
require 'Manage customer accounts' src/critical_pages/customers_portable.rs
require '!is_staff_role(&user.role)' src/critical_pages/customers_portable.rs
require 'Environment operators' src/functional_pages/access_live.rs
require 'is_staff_role(&user.role)' src/functional_pages/access_live.rs
require 'Team will become editable only when the backend has explicit role-management endpoints.' src/functional_pages/access_live.rs

# Customer wallet funding supports normal currency precision and shows the resulting balance before commit.
require 'parse_currency_amount_nano' src/critical_pages/customers_portable.rs
require 'step: "0.01"' src/critical_pages/customers_portable.rs
require 'Review wallet change' src/critical_pages/customers_portable.rs
require 'Balance after' src/critical_pages/customers_portable.rs
forbid 'saturating_mul(1_000_000_000)' src/critical_pages/customers_portable.rs

# API keys are human-owned and lifecycle changes happen in an intentional management flow.
require 'Choose which account will own this router credential' src/functional_pages/access_live.rs
require 'API Key Created' src/functional_pages/access_live.rs
require 'Manage API Key' src/functional_pages/access_live.rs
require 'Credential lifecycle' src/functional_pages/access_live.rs
require 'Delete credential' src/functional_pages/access_live.rs
require 'Rotate API Key' src/functional_pages/access_live.rs
require 'Delete API Key' src/functional_pages/access_live.rs
forbid 'th { "Version" }' src/functional_pages/access_live.rs

# Logs distinguish real timeouts from generic HTTP errors and provide a direct problem-diagnosis path.
require '"problems" => matches!(status, "Timeout" | "Error")' src/functional_pages/logs_full.rs
require 'Problems (error + timeout)' src/functional_pages/logs_full.rs
require 'Show problems' src/functional_pages/logs_full.rs
require 'Inspect' src/functional_pages/logs_full.rs
require 'eq_ignore_ascii_case("timeout")' src/observability.rs
require 'else if self.status_code >= 400' src/observability.rs
forbid 'self.status_code >= 500 ||' src/observability.rs

# Diagnostic pages prioritize conclusions, risks and money before implementation detail.
require 'Failures' src/functional_pages/logs_full.rs
require 'Outcome' src/functional_pages/logs_full.rs
require 'Performance' src/functional_pages/analytics_full.rs
require 'Needs attention' src/functional_pages/analytics_full.rs
require 'What is driving spend' src/functional_pages/analytics.rs
require 'Avg / Request' src/functional_pages/analytics.rs

# Dangerous operational actions require explicit acknowledgement and truthful live state.
require 'confirm_trip' src/functional_pages/guardrails_live.rs
require 'DANGER ZONE' src/functional_pages/guardrails_live.rs
require 'Circuit breaker state is unavailable' src/functional_pages/guardrails_live.rs
require 'confirm_clear' src/functional_pages/settings.rs
require 'MAINTENANCE' src/functional_pages/settings.rs

# Settings distinguishes configured endpoint from verified reachability.
require 'REACHABLE' src/functional_pages/settings.rs
require 'UNVERIFIED' src/functional_pages/settings.rs
require 'Retry runtime check' src/functional_pages/settings.rs
forbid '"CONNECTED"' src/functional_pages/settings.rs

# Chrome must not overclaim runtime health or expose fake top-level utilities.
require 'Server Configured' src/functional_layout.rs
forbid 'Server Connected' src/functional_layout.rs
forbid 'Open request logs' src/functional_layout.rs
forbid 'System settings' src/functional_layout.rs

echo "Product UX contracts OK"
