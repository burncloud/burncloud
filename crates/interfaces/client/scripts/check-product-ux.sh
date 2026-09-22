#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require() {
  local needle="$1"
  local file="$2"
  if [[ ! -f "$file" ]]; then
    echo "Missing product UX file: $file" >&2
    exit 1
  fi
  if ! grep -Fq -- "$needle" "$file"; then
    echo "Missing product UX contract: '$needle' in $file" >&2
    exit 1
  fi
}

line_of() {
  local needle="$1"
  local file="$2"
  grep -nF -- "$needle" "$file" | head -n1 | cut -d: -f1
}

# Buyer navigation follows the product workflow and remains centralized in the
# role-aware shared type rather than duplicated in individual pages.
NAV=src/shared/types/mod.rs
require 'const BUYER_NAV' "$NAV"
require 'key: NavKey::Overview' "$NAV"
require 'key: NavKey::Playground' "$NAV"
require 'key: NavKey::Marketplace' "$NAV"
require 'key: NavKey::ApiKeys' "$NAV"
require 'key: NavKey::Usage' "$NAV"
require 'key: NavKey::Billing' "$NAV"
require 'key: NavKey::Logs' "$NAV"

overview_line=$(line_of 'key: NavKey::Overview' "$NAV")
playground_line=$(line_of 'key: NavKey::Playground' "$NAV")
marketplace_line=$(line_of 'key: NavKey::Marketplace' "$NAV")
api_keys_line=$(line_of 'key: NavKey::ApiKeys' "$NAV")
usage_line=$(line_of 'key: NavKey::Usage' "$NAV")
billing_line=$(line_of 'key: NavKey::Billing' "$NAV")
logs_line=$(line_of 'key: NavKey::Logs' "$NAV")

if ! (( overview_line < playground_line && playground_line < marketplace_line && marketplace_line < api_keys_line && api_keys_line < usage_line && usage_line < billing_line && billing_line < logs_line )); then
  echo "Buyer navigation must remain Overview -> Playground -> Marketplace -> API Keys -> Usage -> Billing -> Logs" >&2
  exit 1
fi

# The shell owns role switching, internationalized navigation, and a usable
# mobile drawer; pages provide their workflows inside that shell.
SHELL=src/shared/layout/mod.rs
require 'pub fn BuyerShell(children: Element)' "$SHELL"
require 'for item in nav_items(current_role)' "$SHELL"
require 'role_menu_open' "$SHELL"
require 'language_menu_open' "$SHELL"
require 'drawer_open' "$SHELL"
require 'copy.select_language' "$SHELL"
require 'aria_haspopup: "true"' "$SHELL"
require 'class: if drawer_open()' "$SHELL"

# Overview exposes health and an actionable path into the two buyer workflows.
OVERVIEW=src/domains/buyer/overview/page.rs
require 'BalanceState::Unknown' "$OVERVIEW"
require 'conclusion-warning' "$OVERVIEW"
require 'href: Some("/buyer/playground".to_string())' "$OVERVIEW"
require 'href: Some("/buyer/marketplace".to_string())' "$OVERVIEW"
require 'role: "alert"' "$OVERVIEW"

# Marketplace supports search, category filtering, an explicit empty state,
# and an accessible details drawer with a direct Playground handoff.
MARKETPLACE=src/domains/buyer/marketplace/page.rs
require 'copy.marketplace_search_placeholder' "$MARKETPLACE"
require 'value.set_search(event.value())' "$MARKETPLACE"
require 'value.set_category(category)' "$MARKETPLACE"
require 'copy.marketplace_no_results' "$MARKETPLACE"
require 'role: "dialog"' "$MARKETPLACE"
require 'aria_modal: "true"' "$MARKETPLACE"
require 'copy.marketplace_test_in_playground' "$MARKETPLACE"
require 'aria_expanded: snapshot.slo_expanded' "$MARKETPLACE"

# Playground lets a buyer inspect request code, vary request controls, run a
# streamed simulation, reset output, and observe request statistics.
PLAYGROUND=src/domains/buyer/playground/page.rs
require 'code_snippet(&snapshot)' "$PLAYGROUND"
require 'role: "tablist"' "$PLAYGROUND"
require 'PlaygroundState::begin_inference' "$PLAYGROUND"
require 'PlaygroundState::clear_output' "$PLAYGROUND"
require 'disabled: snapshot.running' "$PLAYGROUND"
require 'snapshot.stats' "$PLAYGROUND"
require 'copy.playground_response_output' "$PLAYGROUND"

# API-key management requires constrained input, clear errors, a one-time
# secret result, an explicit copy action, and revocation from the table.
API_KEYS=src/domains/buyer/api_keys/page.rs
require 'min: "10"' "$API_KEYS"
require 'max: "5000"' "$API_KEYS"
require 'step: "0.01"' "$API_KEYS"
require 'role: "alert"' "$API_KEYS"
require 'aria_modal: "true"' "$API_KEYS"
require 'navigator.clipboard?.writeText' "$API_KEYS"
require 'value.revoke_key(&key_id)' "$API_KEYS"
require 'copy.api_keys_secret_notice' "$API_KEYS"

# UI copy remains locale-backed, with the selected locale persisted at app
# startup rather than hard-coding a single buyer-facing language.
require 'let copy = strings(locale());' src/domains/buyer/overview/page.rs
require 'let copy = strings(locale());' src/domains/buyer/playground/page.rs
require 'let copy = strings(locale());' src/domains/buyer/marketplace/page.rs
require 'let copy = strings(locale());' src/domains/buyer/api_keys/page.rs
require 'burncloud_selected_language' src/app/app.rs

echo "Product UX contracts OK"
