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
evaluation_line=$(line_of 'NavItem { to:Route::Evaluation' src/functional_layout.rs)
billing_line=$(line_of 'NavItem { to:Route::Billing' src/functional_layout.rs)

if ! (( providers_line < models_line && models_line < routes_line && routes_line < playground_line )); then
  echo "Traffic Setup navigation must remain Providers -> Models -> Routes -> Playground" >&2
  exit 1
fi
if ! (( logs_line < evaluation_line && evaluation_line < billing_line )); then
  echo "Observe navigation must remain Logs -> Evaluation -> Billing" >&2
  exit 1
fi

# Overview must answer readiness and next action before technical diagnostics.
require 'Setup & readiness' src/critical_pages/dashboard.rs
require 'BurnCloud is ready to serve traffic' src/critical_pages/dashboard.rs
require 'Finish setup before sending production traffic' src/critical_pages/dashboard.rs
require 'Add first provider' src/critical_pages/dashboard.rs
require 'Test a request' src/critical_pages/dashboard.rs

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
require 'Environment operators' src/functional_pages/access_live.rs
require 'is_staff_role(&user.role)' src/functional_pages/access_live.rs
require 'Team will become editable only when the backend has explicit role-management endpoints.' src/functional_pages/access_live.rs

# API keys must be human-owned and lifecycle/destructive actions need explicit steps.
require 'Choose which account will own this router credential' src/functional_pages/access_live.rs
require 'API Key Created' src/functional_pages/access_live.rs
require 'Rotate API Key' src/functional_pages/access_live.rs
require 'Delete API Key' src/functional_pages/access_live.rs

# Diagnostic pages prioritize conclusions and risks.
require 'Failures' src/functional_pages/logs_full.rs
require 'Outcome' src/functional_pages/logs_full.rs
require 'Operational attention' src/functional_pages/analytics_full.rs
require 'Spend by model' src/functional_pages/analytics.rs

# Dangerous operational actions require explicit acknowledgement and stay in danger zones.
require 'confirm_trip' src/functional_pages/guardrails_live.rs
require 'DANGER ZONE' src/functional_pages/guardrails_live.rs
require 'confirm_clear' src/functional_pages/settings.rs
require 'MAINTENANCE' src/functional_pages/settings.rs

# Chrome must not overclaim runtime health.
require 'Server Configured' src/functional_layout.rs
forbid 'Server Connected' src/functional_layout.rs

echo "Product UX contracts OK"
