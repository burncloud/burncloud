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

# All protected console routes must use the functional page module, not the static prototype exports.
require 'functional_pages::{' src/app.rs
require 'auth_gate::AuthGate' src/app.rs
require 'FunctionalConsoleLayout' src/auth_gate.rs
require 'auth.clear();' src/functional_layout.rs
require 'pub use providers::Providers;' src/functional_pages/mod.rs
require 'pub use catalog::{Models, Routes};' src/functional_pages/mod.rs
require 'pub use logs_full::Logs;' src/functional_pages/mod.rs
require 'pub use analytics_full::Evaluation;' src/functional_pages/mod.rs

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

# Page-to-service contracts: these make accidental regressions back to seeded/static pages fail CI.
require 'AuthService::login' src/critical_pages/auth.rs
require 'AuthService::register' src/critical_pages/auth.rs
require 'UserService::list' src/critical_pages/customers_portable.rs
require 'UserService::topup' src/critical_pages/customers_portable.rs
require 'billing_summary' src/critical_pages/dashboard.rs
require 'ChannelService::list' src/critical_pages/dashboard.rs
require 'TokenService::create' src/functional_pages/access_live.rs
require 'ChannelService::create' src/functional_pages/providers.rs
require 'update_channel_preserving_reservations' src/functional_pages/providers.rs
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
