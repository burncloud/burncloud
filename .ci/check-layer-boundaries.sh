#!/usr/bin/env bash
# Layer boundary enforcement for crates/server
#
# Rule: server may only depend on database crates for init-only schema migration.
# All business data access must go through the service layer.
#
# Allowed server → database exceptions (schema migration at startup):
#   - burncloud-database          (connection pool + migration runner)
#   - burncloud-database-router   (router table schema init)
#   - burncloud-database-user     (user table schema init)
#
# Everything else in burncloud-database-* is banned from server's direct deps.

set -euo pipefail

SERVER_CARGO="crates/server/Cargo.toml"

ALLOWED=(
    "burncloud-database"
    "burncloud-database-router"
    "burncloud-database-user"
)

# Collect all burncloud-database-* direct deps in server/Cargo.toml
FOUND_VIOLATIONS=()

while IFS= read -r line; do
    # Match lines like: burncloud-database-foo = ...  or  burncloud-database-foo.workspace = true
    if [[ "$line" =~ ^[[:space:]]*(burncloud-database-[a-z-]+) ]]; then
        dep="${BASH_REMATCH[1]}"
        # Check if this dep is in the allowed list
        allowed=false
        for a in "${ALLOWED[@]}"; do
            if [[ "$dep" == "$a" ]]; then
                allowed=true
                break
            fi
        done
        if [[ "$allowed" == "false" ]]; then
            FOUND_VIOLATIONS+=("$dep")
        fi
    fi
done < "$SERVER_CARGO"

# Also check for the bare burncloud-database-* pattern (catches burncloud-database-models etc)
# but make sure burncloud-database itself (exact match) is not flagged
while IFS= read -r line; do
    if [[ "$line" =~ ^[[:space:]]*(burncloud-database)([^-]|$) ]]; then
        # exact "burncloud-database" — this is allowed, skip
        :
    fi
done < "$SERVER_CARGO"

if [[ ${#FOUND_VIOLATIONS[@]} -gt 0 ]]; then
    echo "❌ Layer boundary violation: crates/server directly depends on database crates"
    echo ""
    echo "Banned deps found in ${SERVER_CARGO}:"
    for v in "${FOUND_VIOLATIONS[@]}"; do
        echo "  - $v"
    done
    echo ""
    echo "Server must access data through the service layer (burncloud-service-*)."
    echo "Allowed exceptions (schema migration only): ${ALLOWED[*]}"
    echo ""
    echo "To fix: move the dependency to the appropriate service crate,"
    echo "or add a new service crate if one does not exist."
    exit 1
fi

echo "✅ Layer boundary check passed — no banned database deps in crates/server"
