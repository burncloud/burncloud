#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require() {
  local needle="$1"
  local file="$2"
  if [[ ! -f "$file" ]]; then
    echo "Missing functional wiring file: $file" >&2
    exit 1
  fi
  if ! grep -Fq -- "$needle" "$file"; then
    echo "Missing functional wiring: '$needle' in $file" >&2
    exit 1
  fi
}

# The client was migrated from flat `functional_pages` modules to the app /
# domains / shared layout. Keep the checks aligned with the current tree.
require 'Router::<Route> {}' src/app/app.rs
require 'pub use app::App;' src/app/mod.rs
require 'pub mod buyer;' src/domains/mod.rs
require 'pub mod api_keys;' src/domains/buyer/mod.rs
require 'pub mod marketplace;' src/domains/buyer/mod.rs
require 'pub mod overview;' src/domains/buyer/mod.rs
require 'pub mod playground;' src/domains/buyer/mod.rs

# Buyer pages must be imported, routed, and rendered by the Dioxus router.
ROUTES=src/app/router/routes.rs
require 'api_keys::BuyerApiKeys' "$ROUTES"
require 'marketplace::BuyerMarketplace' "$ROUTES"
require 'overview::BuyerOverview' "$ROUTES"
require 'playground::BuyerPlayground' "$ROUTES"
require '#[route("/buyer/overview")]' "$ROUTES"
require '#[route("/buyer/playground")]' "$ROUTES"
require '#[route("/buyer/marketplace")]' "$ROUTES"
require '#[route("/buyer/api-keys")]' "$ROUTES"
require 'rsx! { BuyerOverview {} }' "$ROUTES"
require 'rsx! { BuyerPlayground {} }' "$ROUTES"
require 'rsx! { BuyerMarketplace {} }' "$ROUTES"
require 'rsx! { BuyerApiKeys {} }' "$ROUTES"

require 'pub use page::BuyerOverview;' src/domains/buyer/overview/mod.rs
require 'pub use page::BuyerPlayground;' src/domains/buyer/playground/mod.rs
require 'pub use page::BuyerMarketplace;' src/domains/buyer/marketplace/mod.rs
require 'pub use page::BuyerApiKeys;' src/domains/buyer/api_keys/mod.rs
require 'BuyerShell {' src/domains/buyer/overview/page.rs
require 'BuyerShell {' src/domains/buyer/playground/page.rs
require 'BuyerShell {' src/domains/buyer/marketplace/page.rs
require 'BuyerShell {' src/domains/buyer/api_keys/page.rs

# API key management must retain creation, validation, one-time disclosure,
# and revocation state instead of regressing to static rows.
API_ACTIONS=src/domains/buyer/api_keys/actions.rs
API_STATE=src/domains/buyer/api_keys/state.rs
API_PAGE=src/domains/buyer/api_keys/page.rs
require 'pub fn create_api_key(' "$API_ACTIONS"
require 'pub fn revoke_api_key(' "$API_ACTIONS"
require 'pub fn validate_create_input(' "$API_ACTIONS"
require 'pub fn generate_secret()' "$API_ACTIONS"
require 'pub fn create_key(&mut self)' "$API_STATE"
require 'pub fn revoke_key(&mut self, id: &str)' "$API_STATE"
require 'created_secret: Option<String>' "$API_STATE"
require 'aria_modal: "true"' "$API_PAGE"
require 'ApiKeysState::mark_copied' "$API_PAGE"

# Marketplace filtering and details remain driven by domain state and model data.
MARKETPLACE_MODEL=src/domains/buyer/marketplace/model.rs
MARKETPLACE_STATE=src/domains/buyer/marketplace/state.rs
MARKETPLACE_PAGE=src/domains/buyer/marketplace/page.rs
require 'pub const MODEL_CATALOG' "$MARKETPLACE_MODEL"
require 'pub fn filtered_models(&self)' "$MARKETPLACE_STATE"
require 'pub fn open_details(&mut self' "$MARKETPLACE_STATE"
require 'pub fn toggle_slo(&mut self)' "$MARKETPLACE_STATE"
require 'MarketplaceState::default' "$MARKETPLACE_PAGE"
require 'value.set_search(event.value())' "$MARKETPLACE_PAGE"
require 'value.set_category(category)' "$MARKETPLACE_PAGE"
require 'aria_modal: "true"' "$MARKETPLACE_PAGE"
require 'aria_expanded: snapshot.slo_expanded' "$MARKETPLACE_PAGE"

# The Playground must continue to produce an API client example and model a
# cancellable streaming request rather than presenting an inert prototype.
PLAYGROUND_ACTIONS=src/domains/buyer/playground/actions.rs
PLAYGROUND_STATE=src/domains/buyer/playground/state.rs
PLAYGROUND_PAGE=src/domains/buyer/playground/page.rs
require 'pub fn code_snippet(state: &PlaygroundState)' "$PLAYGROUND_ACTIONS"
require '/v1/chat/completions' "$PLAYGROUND_ACTIONS"
require 'pub fn begin_inference(&mut self)' "$PLAYGROUND_STATE"
require 'pub fn append_stream_chunk(&mut self' "$PLAYGROUND_STATE"
require 'pub fn complete_inference(&mut self' "$PLAYGROUND_STATE"
require 'STREAMED_RESPONSE' "$PLAYGROUND_PAGE"
require 'disabled: snapshot.running' "$PLAYGROUND_PAGE"
require 'PlaygroundState::clear_output' "$PLAYGROUND_PAGE"

# The shared shell and LiveView endpoint are part of the rendered product.
require 'pub fn BuyerShell(children: Element)' src/shared/layout/mod.rs
require 'for item in nav_items(current_role)' src/shared/layout/mod.rs
require 'GlobalStyle {}' src/shared/layout/mod.rs
require 'let locale = use_context_provider' src/app/app.rs
require 'localStorage.getItem' src/app/app.rs
require 'pub fn liveview_router' src/lib.rs
require '"/console/ws"' src/lib.rs
require '.route("/buyer/{*route}", get(index.clone()))' src/lib.rs

echo "Functional console wiring OK"
