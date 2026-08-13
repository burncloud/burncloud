#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require() {
  local needle="$1"
  local file="$2"
  if ! grep -Fq "$needle" "$file"; then
    echo "Missing functional wiring: '$needle' in $file" >&2
    exit 1
  fi
}

# All protected console routes must use the functional page modules, not static prototype exports.
require 'functional_pages::{' src/app.rs
require 'auth_gate::AuthGate' src/app.rs
require 'FunctionalConsoleLayout' src/auth_gate.rs
require 'auth.clear();' src/functional_layout.rs
require 'pub use providers::Providers;' src/functional_pages/mod.rs
require 'pub use catalog::{Models, Routes};' src/functional_pages/mod.rs
require 'pub use logs_full::Logs;' src/functional_pages/mod.rs
require 'pub use analytics_full::Evaluation;' src/functional_pages/mod.rs
require 'pub use team_live::Team;' src/functional_pages/mod.rs
require 'pub use overview_live::Overview;' src/critical_pages.rs

# Public runtime routes must use the truthful public module, never the historical prototype page module.
require 'public_pages::{Home, Landing}' src/app.rs
require 'pub mod public_pages;' src/lib.rs
if grep -Fq 'pages::{Home, Landing}' src/app.rs || grep -Fq 'pub mod pages;' src/lib.rs; then
  echo "Historical prototype pages were reconnected to the runtime" >&2
  exit 1
fi

# Historical overview/team implementations must not remain in the active module graph.
if grep -Fq 'mod dashboard;' src/critical_pages.rs; then
  echo "Historical dashboard implementation was reconnected to the runtime" >&2
  exit 1
fi
if grep -Fq 'pub use access_live::{APIKeys, Team};' src/functional_pages/mod.rs; then
  echo "Historical Team implementation was reconnected to API-key module" >&2
  exit 1
fi

# Authentication and persistent session wiring.
require '/api/auth/login' src/backend.rs
require '/api/auth/register' src/backend.rs
require '/api/auth/forgot-password' src/backend.rs
require 'Authorization' src/backend.rs

# Real console API groups used by the rebuilt pages.
require '/console/api/list_users' src/backend.rs
require '/console/api/user/register' src/backend.rs
require '/console/api/user/topup' src/backend.rs
require '/console/api/channel?limit=' src/backend.rs
require '/console/api/tokens' src/backend.rs
require '/console/api/usage/' src/backend.rs
require '/api/billing/summary' src/backend.rs
require '/console/api/monitor' src/backend.rs
require '/v1/chat/completions' src/backend.rs
require '/console/api/logs?page=1&page_size=' src/observability.rs
require 'video_tokens' src/observability.rs
require 'audio_input_tokens' src/observability.rs
require 'image_tokens' src/observability.rs
require 'embedding_tokens' src/observability.rs
require '/console/api/monitor/security/filters' src/functional_api.rs
require '/console/api/monitor/security/events' src/functional_api.rs
require '/console/api/monitor/security/emergency-circuit-break' src/functional_api.rs
require '/console/api/cache/stats' src/functional_api.rs
require '/console/api/cache/clear' src/functional_api.rs
require 'reservation_green' src/functional_api.rs
require 'reservation_yellow' src/functional_api.rs
require 'reservation_red' src/functional_api.rs
require 'current_status != 1' src/functional_api.rs
require 'repair_channel_and_reactivate' src/functional_api.rs
require 'allow_reactivation' src/functional_api.rs

# Page-to-service contracts: accidental regressions back to seeded/static pages fail CI.
require 'AuthService::login' src/critical_pages/auth.rs
require 'AuthService::register' src/critical_pages/auth.rs
require 'UserService::list' src/critical_pages/customers_portable.rs
require 'UserService::topup' src/critical_pages/customers_portable.rs
require 'billing_summary' src/critical_pages/overview_live.rs
require 'user_usage' src/critical_pages/overview_live.rs
require 'ChannelService::list' src/critical_pages/overview_live.rs
require 'LogService::list' src/critical_pages/overview_live.rs
require 'UserService::list' src/functional_pages/team_live.rs
require 'TokenService::create' src/functional_pages/access_live.rs
require 'ChannelService::create' src/functional_pages/providers.rs
require 'update_channel_preserving_reservations' src/functional_pages/providers.rs
require 'repair_channel_and_reactivate' src/functional_pages/providers.rs
require 'ChannelService::list' src/functional_pages/catalog.rs
require 'full_logs' src/functional_pages/logs_full.rs
require 'full_logs' src/functional_pages/analytics_full.rs
require 'chat_completion' src/functional_pages/playground_live.rs
require 'save_security_filters' src/functional_pages/guardrails_live.rs
require 'clear_cache' src/functional_pages/settings.rs
require 'billing_summary' src/functional_pages/analytics.rs

# Unsupported prototype actions must not reappear as fake success paths.
if grep -Fq 'Suspend Account' src/critical_pages/customers_portable.rs; then
  echo "Fake suspend action reintroduced without a server endpoint" >&2
  exit 1
fi
if grep -Fq 'Prompt Snippet' src/functional_pages/logs_full.rs; then
  echo "Synthetic prompt content reintroduced into router log UI" >&2
  exit 1
fi

# Chrome controls should either navigate/act or be semantic status elements.
require 'search_route(&query)' src/functional_layout.rs
require 'div { class:"env-chip"' src/functional_layout.rs

echo "Functional console wiring OK"
