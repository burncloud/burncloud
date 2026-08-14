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

# The Console has one product spine: Workspace -> Supply -> Access -> Verify/Observe -> Govern.
require 'NavGroup { title:"Workspace"' src/functional_layout.rs
require 'NavGroup { title:"Supply"' src/functional_layout.rs
require 'NavGroup { title:"Access"' src/functional_layout.rs
require 'NavGroup { title:"Verify & Observe"' src/functional_layout.rs
require 'NavGroup { title:"Govern"' src/functional_layout.rs

overview_line=$(line_of 'NavItem { to:Route::Overview' src/functional_layout.rs)
providers_line=$(line_of 'NavItem { to:Route::Providers' src/functional_layout.rs)
models_line=$(line_of 'NavItem { to:Route::Models' src/functional_layout.rs)
routes_line=$(line_of 'NavItem { to:Route::Routes' src/functional_layout.rs)
customers_line=$(line_of 'NavItem { to:Route::Customers' src/functional_layout.rs)
keys_line=$(line_of 'NavItem { to:Route::APIKeys' src/functional_layout.rs)
playground_line=$(line_of 'NavItem { to:Route::Playground' src/functional_layout.rs)
logs_line=$(line_of 'NavItem { to:Route::Logs' src/functional_layout.rs)
evaluation_line=$(line_of 'NavItem { to:Route::Evaluation' src/functional_layout.rs)
billing_line=$(line_of 'NavItem { to:Route::Billing' src/functional_layout.rs)
guardrails_line=$(line_of 'NavItem { to:Route::Guardrails' src/functional_layout.rs)
team_line=$(line_of 'NavItem { to:Route::Team' src/functional_layout.rs)
settings_line=$(line_of 'NavItem { to:Route::Settings' src/functional_layout.rs)

if ! (( overview_line < providers_line && providers_line < models_line && models_line < routes_line )); then
  echo "Product flow must start Overview -> Providers -> Models -> Routes" >&2
  exit 1
fi
if ! (( routes_line < customers_line && customers_line < keys_line && keys_line < playground_line )); then
  echo "Product flow must hand off Supply -> Customers -> API Keys -> Playground" >&2
  exit 1
fi
if ! (( playground_line < logs_line && logs_line < evaluation_line && evaluation_line < billing_line )); then
  echo "Verification/observation must remain Playground -> Logs -> Evaluation -> Billing" >&2
  exit 1
fi
if ! (( billing_line < guardrails_line && guardrails_line < team_line && team_line < settings_line )); then
  echo "Govern navigation must remain Guardrails -> Team -> Settings" >&2
  exit 1
fi

# Overview is a conclusion + handoff surface. It must not become a second Providers/Logs/Billing/Settings page.
require 'Evidence-backed overview' src/critical_pages/dashboard.rs
require 'Product flow evidence' src/critical_pages/dashboard.rs
require 'Building an evidence-backed overview' src/critical_pages/dashboard.rs
require 'Overview evidence is incomplete' src/critical_pages/dashboard.rs
require 'Configuration is present; verification is still missing' src/critical_pages/dashboard.rs
require 'Verified traffic is observable' src/critical_pages/dashboard.rs
require 'Unknown data stays unknown instead of becoming a zero or healthy state.' src/critical_pages/dashboard.rs
require 'Needs attention' src/critical_pages/dashboard.rs
require 'Observed activity' src/critical_pages/dashboard.rs
require 'Latest request evidence' src/critical_pages/dashboard.rs
require 'Responsibility boundary:' src/critical_pages/dashboard.rs
require 'Open Providers' src/critical_pages/dashboard.rs
require 'Open Models' src/critical_pages/dashboard.rs
require 'Create API key' src/critical_pages/dashboard.rs
require 'Open Playground' src/critical_pages/dashboard.rs
require 'Open Logs' src/critical_pages/dashboard.rs
forbid 'routing_configured' src/critical_pages/dashboard.rs
forbid 'Provider health' src/critical_pages/dashboard.rs
forbid 'Stored Request Route Receipt' src/critical_pages/dashboard.rs
forbid 'Host resource pressure' src/critical_pages/dashboard.rs
forbid 'Top billed models' src/critical_pages/dashboard.rs

# Auth must expose only current backend capabilities, not prototype choices.
forbid 'Onboarding Account Preference' src/critical_pages/auth.rs
forbid 'TierButton' src/critical_pages/auth.rs
forbid 'Company / Team' src/critical_pages/auth.rs
forbid 'auth-tabs' src/critical_pages/auth.rs
require 'Password recovery' src/critical_pages/auth.rs
require 'Account email' src/critical_pages/auth.rs

# Providers present product concepts instead of raw enum IDs and protect destructive changes.
require 'PROVIDER_TYPES' src/functional_pages/providers.rs
require 'Provider type' src/functional_pages/providers.rs
require 'Advanced routing & capacity' src/functional_pages/providers.rs
require 'Leave blank to keep stored credential' src/functional_pages/providers.rs
require 'pending_delete' src/functional_pages/providers.rs
forbid 'Provider Type ID' src/functional_pages/providers.rs

# Models/Routes communicate service availability and resilience, not just database fields.
require 'Single upstream' src/functional_pages/catalog.rs
require 'Redundant' src/functional_pages/catalog.rs
require 'Unavailable' src/functional_pages/catalog.rs
require 'No failover redundancy' src/functional_pages/catalog.rs

# Playground is a guided end-to-end test and blocks impossible workflows.
require 'Playground is not ready yet' src/functional_pages/playground_live.rs
require 'Connect an active provider first' src/functional_pages/playground_live.rs
require 'Create an API key for the test' src/functional_pages/playground_live.rs
require 'Select a configured model' src/functional_pages/playground_live.rs
require 'Send Test Request' src/functional_pages/playground_live.rs

# Customers and staff have separate product responsibilities.
require 'Manage business accounts' src/critical_pages/customers_portable.rs
require '!is_staff_role(&user.role)' src/critical_pages/customers_portable.rs
require 'Loading customer accounts' src/critical_pages/customers_portable.rs
require 'Default Status' src/critical_pages/customers_portable.rs
require 'Status is server metadata.' src/critical_pages/customers_portable.rs
require 'parse_positive_amount_nano' src/critical_pages/customers_portable.rs
require 'Use no more than two decimal places.' src/critical_pages/customers_portable.rs
require 'Funding review' src/critical_pages/customers_portable.rs
require 'New {selected_currency} balance' src/critical_pages/customers_portable.rs
forbid '"Disabled"' src/critical_pages/customers_portable.rs
forbid 'enabled accounts' src/critical_pages/customers_portable.rs
forbid 'saturating_mul(1_000_000_000)' src/critical_pages/customers_portable.rs
require 'Environment operators' src/functional_pages/access_live.rs
require 'is_staff_role(&user.role)' src/functional_pages/access_live.rs
require 'Team will become editable only when the backend has explicit role-management endpoints.' src/functional_pages/access_live.rs

# API-key management must keep opaque management references separate from bearer-secret disclosure.
require 'Opaque management reference' src/functional_pages/api_keys_live.rs
require 'not a masked bearer secret' src/functional_pages/api_keys_live.rs
require 'New key creation is unavailable' src/functional_pages/api_keys_live.rs
require 'USD spend limit' src/functional_pages/api_keys_live.rs
require 'One-time bearer secret' src/functional_pages/api_keys_live.rs
require 'I saved this credential' src/functional_pages/api_keys_live.rs
require 'CIDR ranges are not supported' src/functional_pages/api_keys_live.rs
require 'Rotate API Key' src/functional_pages/api_keys_live.rs
require 'Delete API Key' src/functional_pages/api_keys_live.rs
forbid 'Owner user ID' src/functional_pages/api_keys_live.rs
forbid 'fn masked' src/functional_pages/api_keys_live.rs

# Diagnostic pages prioritize conclusions and risks.
require 'Failures' src/functional_pages/logs_full.rs
require 'Outcome' src/functional_pages/logs_full.rs
require 'Operational attention' src/functional_pages/analytics_full.rs
require 'Spend by model' src/functional_pages/analytics.rs

# Guardrails must describe HTTP-error-derived evidence truthfully and fail closed on unknown policy state.
require 'Request Health' src/functional_pages/guardrails_live.rs
require 'HTTP risk signals' src/functional_pages/guardrails_live.rs
require 'not a threat-intelligence feed' src/functional_pages/guardrails_live.rs
require 'BurnCloud will not show default-off controls' src/functional_pages/guardrails_live.rs
require 'Circuit breaker state is unavailable' src/functional_pages/guardrails_live.rs
require 'Save Protection Policy' src/functional_pages/guardrails_live.rs
forbid 'Security Score' src/functional_pages/guardrails_live.rs
forbid 'Threat Sources' src/functional_pages/guardrails_live.rs
forbid 'Circuit breaker telemetry connected' src/functional_pages/guardrails_live.rs

# Dangerous operational actions require explicit acknowledgement and stay in danger zones.
require 'confirm_trip' src/functional_pages/guardrails_live.rs
require 'DANGER ZONE' src/functional_pages/guardrails_live.rs
require 'confirm_clear' src/functional_pages/settings.rs
require 'MAINTENANCE' src/functional_pages/settings.rs

# Chrome must not overclaim runtime health.
require 'Server Configured' src/functional_layout.rs
forbid 'Server Connected' src/functional_layout.rs

echo "Product UX contracts OK"
