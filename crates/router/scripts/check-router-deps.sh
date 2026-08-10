#!/usr/bin/env bash
# check-router-deps.sh — Verify burncloud-router only depends on whitelisted service crates.
#
# Current architecture rule: router may depend on service-billing and
# service-user, but no other burncloud-service-* crate. Adding a new service
# dependency requires architecture review and an explicit whitelist update.
#
# Usage:
#   ./crates/router/scripts/check-router-deps.sh          # human-readable output
#   ./crates/router/scripts/check-router-deps.sh --ci     # CI-friendly (no color)

set -euo pipefail

CI_MODE=false
if [[ "${1:-}" == "--ci" ]]; then
  CI_MODE=true
fi

RED=""
GREEN=""
RESET=""
if [[ "$CI_MODE" == "false" ]]; then
  RED=$(tput setaf 1 2>/dev/null || echo "")
  GREEN=$(tput setaf 2 2>/dev/null || echo "")
  RESET=$(tput sgr0 2>/dev/null || echo "")
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# ── Whitelist: service crates that router is allowed to depend on ──
ALLOWED_SERVICE_CRATES=(
  burncloud-service-billing
  burncloud-service-user
)

cd "$REPO_ROOT"

for cmd in cargo jq; do
  if ! command -v "$cmd" &>/dev/null; then
    echo "${RED}Error: $cmd is required but not installed${RESET}"
    exit 1
  fi
done

CARGO_METADATA=$(cargo metadata --format-version 1 --no-deps)

if ! echo "$CARGO_METADATA" | jq -e '.packages[] | select(.name == "burncloud-router")' > /dev/null; then
  echo "${RED}Error: could not find burncloud-router in cargo metadata${RESET}"
  echo "Make sure you are running this from the workspace root."
  exit 1
fi

SERVICE_DEPS=()
for dep in $(echo "$CARGO_METADATA" | jq -r '.packages[] | select(.name == "burncloud-router") | .dependencies[] | .name | select(startswith("burncloud-service-"))'); do
  SERVICE_DEPS+=("$dep")
done

VIOLATIONS=()
for dep in "${SERVICE_DEPS[@]}"; do
  if ! [[ " ${ALLOWED_SERVICE_CRATES[*]} " =~ " $dep " ]]; then
    VIOLATIONS+=("$dep")
  fi
done

if [[ ${#VIOLATIONS[@]} -eq 0 ]]; then
  allowed_list="${ALLOWED_SERVICE_CRATES[*]}"
  echo "${GREEN}OK: burncloud-router service dependencies are within the architecture whitelist.${RESET}"
  echo "  Allowed: $allowed_list"
  echo "  Found:   ${SERVICE_DEPS[*]:-none}"
  exit 0
fi

allowed_str=$(printf '%s, ' "${ALLOWED_SERVICE_CRATES[@]}")
allowed_str="${allowed_str%, }"

echo "${RED}Architecture violation: burncloud-router depends on unauthorized service crate(s)${RESET}"
echo ""
echo "  Found:   ${VIOLATIONS[*]}"
echo "  Allowed: $allowed_str"
echo ""
echo "  The current router service-dependency boundary is enforced by this script."
echo "  Adding a new burncloud-service-* dependency requires architecture review"
echo "  and an explicit update to the whitelist if the new dependency is accepted."
echo ""
echo "  See: docs/agent/INVARIANTS.md"
echo "  See: docs/contracts/ROUTER.md"
echo "  Or:  crates/router/README.md \"Dependency boundary\" section"
exit 1
