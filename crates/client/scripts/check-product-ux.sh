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

# Overview must keep unknown data unknown, separate account activity from environment health, and require real traffic evidence.
require 'Setup & verification' src/critical_pages/overview_live.rs
require 'Checking this environment' src/critical_pages/overview_live.rs
require 'Unknown values stay unknown instead of being displayed as zero' src/critical_pages/overview_live.rs
require 'Your usage' src/critical_pages/overview_live.rs
require 'Your Requests' src/critical_pages/overview_live.rs
require 'Your Tokens' src/critical_pages/overview_live.rs
require 'Your Spend' src/critical_pages/overview_live.rs
require 'Environment health' src/critical_pages/overview_live.rs
require 'Recent successful request observed' src/critical_pages/overview_live.rs
require 'Traffic path verified' src/critical_pages/overview_live.rs
require 'router_status_label' src/critical_pages/overview_live.rs
require 'eq_ignore_ascii_case("timeout")' src/critical_pages/overview_live.rs
require 'pub use overview_live::Overview;' src/critical_pages.rs
forbid 'mod dashboard;' src/critical_pages.rs
forbid 'BurnCloud is ready to serve traffic' src/critical_pages/overview_live.rs
forbid 'System Overview' src/critical_pages/overview_live.rs

# Auth exposes real account capabilities only, chooses the workspace from roles, and avoids fake legal/developer UI.
forbid 'Onboarding Account Preference' src/critical_pages/auth.rs
forbid 'TierButton' src/critical_pages/auth.rs
forbid 'Company / Team' src/critical_pages/auth.rs
forbid 'auth-tabs' src/critical_pages/auth.rs
require 'Password recovery' src/critical_pages/auth.rs
require 'Account email' src/critical_pages/auth.rs
require 'Sign in to the BurnCloud workspace available to your account.' src/critical_pages/auth.rs
require 'is_staff_roles(&response.roles)' src/critical_pages/auth.rs
require 'Route::Billing' src/critical_pages/auth.rs
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
require 'Repair Provider' src/functional_pages/providers.rs
require 'Saving this repair will reactivate routing' src/functional_pages/providers.rs
require 'Save & Reactivate' src/functional_pages/providers.rs
require 'repair_channel_and_reactivate' src/functional_pages/providers.rs
require 'explicit repair flow' src/functional_api.rs
forbid 'Provider Type ID' src/functional_pages/providers.rs
forbid 'P{} • W{}' src/functional_pages/providers.rs
forbid '∞ RPM' src/functional_pages/providers.rs
forbid 'disabled: channel.status != 1' src/functional_pages/providers.rs

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

# Customers and admins have separate product responsibilities under the current admin/user role model.
require 'Manage customer accounts' src/critical_pages/customers_portable.rs
require '!is_staff_role(&user.role)' src/critical_pages/customers_portable.rs
require 'role_access::is_staff_role' src/functional_pages/team_live.rs
require 'Admin directory' src/functional_pages/team_live.rs
require 'Current session appears in the admin directory' src/functional_pages/team_live.rs
require 'current persisted role model exposes admin and user roles' src/functional_pages/team_live.rs
require 'This page is an access inventory, not a fake staff-management screen.' src/functional_pages/team_live.rs
require 'pub use team_live::Team;' src/functional_pages/mod.rs
forbid 'pub use access_live::{APIKeys, Team};' src/functional_pages/mod.rs
forbid 'Invite Organization Member' src/functional_pages/team_live.rs
forbid 'Send Secure Invitation' src/functional_pages/team_live.rs
forbid 'admin / owner / operator identities' src/functional_pages/team_live.rs

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

# Guardrails must describe backend-derived HTTP error signals accurately instead of overclaiming security intelligence.
require 'Request Health' src/functional_pages/guardrails_live.rs
require 'HTTP Error Events' src/functional_pages/guardrails_live.rs
require 'Affected IDs / Upstreams' src/functional_pages/guardrails_live.rs
require 'operational traffic indicators, not a threat-intelligence feed' src/functional_pages/guardrails_live.rs
require 'BurnCloud will not show default-off controls when the real saved policy could not be loaded.' src/functional_pages/guardrails_live.rs
require 'HTTP risk signals' src/functional_pages/guardrails_live.rs
require 'A 4xx does not automatically mean a malicious client' src/functional_pages/guardrails_live.rs
require 'confirm_trip' src/functional_pages/guardrails_live.rs
require 'DANGER ZONE' src/functional_pages/guardrails_live.rs
require 'Circuit breaker state is unavailable' src/functional_pages/guardrails_live.rs
forbid 'Security Score' src/functional_pages/guardrails_live.rs
forbid 'Threat Sources' src/functional_pages/guardrails_live.rs

# Diagnostic pages prioritize truthful sample/account semantics, risk, and money before implementation detail.
require 'Failures' src/functional_pages/logs_full.rs
require 'Outcome' src/functional_pages/logs_full.rs
require 'Performance' src/functional_pages/analytics_full.rs
require 'Sample observations' src/functional_pages/analytics_full.rs
require 'Single upstream observed' src/functional_pages/analytics_full.rs
require 'What is driving this account' src/functional_pages/analytics.rs
require 'Account scope:' src/functional_pages/analytics.rs
require 'Avg / Request' src/functional_pages/analytics.rs

# Dangerous operational actions require explicit acknowledgement and truthful live state.
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
