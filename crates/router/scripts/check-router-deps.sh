#!/usr/bin/env bash
# check-router-deps.sh — Verify burncloud-router only depends on whitelisted service crates.
#
# Constitutional exception (§1.1): router may depend on service-billing and
# service-user, but no other burncloud-service-* crate. Adding a new service
# dependency requires architecture review.
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
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# ── Whitelist: service crates that router is allowed to depend on ──
ALLOWED_SERVICE_CRATES=(
  burncloud-service-billing
  burncloud-service-user
)

# ── Extract burncloud-router's direct dependencies via cargo metadata ──
cd "$REPO_ROOT"

DEPS_JSON=$(cargo metadata --format-version 1 --no-deps 2>/dev/null | \
  jq '.packages[] | select(.name == "burncloud-router") | .dependencies[] | .name')

if [[ -z "$DEPS_JSON" ]]; then
  echo "${RED}Error: could not find burncloud-router in cargo metadata${RESET}"
  echo "Make sure you are running this from the workspace root."
  exit 1
fi

# ── Find all burncloud-service-* dependencies ──
SERVICE_DEPS=()
for dep in $DEPS_JSON; do
  dep=$(echo "$dep" | tr -d '"')
  if [[ "$dep" == burncloud-service-* ]]; then
    SERVICE_DEPS+=("$dep")
  fi
done

# ── Check each service dep against the whitelist ──
VIOLATIONS=()
for dep in "${SERVICE_DEPS[@]:-}"; do
  allowed=false
  for ok in "${ALLOWED_SERVICE_CRATES[@]}"; do
    if [[ "$dep" == "$ok" ]]; then
      allowed=true
      break
    fi
  done
  if [[ "$allowed" == "false" ]]; then
    VIOLATIONS+=("$dep")
  fi
done

# ── Report ──
if [[ ${#VIOLATIONS[@]} -eq 0 ]]; then
  allowed_list="${ALLOWED_SERVICE_CRATES[*]}"
  echo "${GREEN}OK: burncloud-router service dependencies are within constitutional exception whitelist.${RESET}"
  echo "  Allowed: $allowed_list"
  echo "  Found:   ${SERVICE_DEPS[*]:-none}"
  exit 0
fi

# Build the allowed list string for the error message
allowed_str=$(printf '%s, ' "${ALLOWED_SERVICE_CRATES[@]}")
allowed_str="${allowed_str%, }"

echo "${RED}Architecture violation: burncloud-router depends on unauthorized service crate(s)${RESET}"
echo ""
echo "  Found:   ${VIOLATIONS[*]}"
echo "  Allowed: $allowed_str"
echo ""
echo "  Router is a data-plane component. Dependency direction should be"
echo "  Router -> Database -> Common. Service-* dependencies are constitutional"
echo "  exceptions; adding new ones requires architecture review."
echo ""
echo "  See: docs/code/README.md §1.1 \"Constitutional Exceptions\""
echo "  Or:  crates/router/README.md \"Dependencies\" section"
exit 1