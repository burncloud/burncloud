#!/usr/bin/env bash
# Fail when console UI uses raw <button class="btn ..."> instead of BCButton.
# Guest / client-api crates are excluded (see docs/ui/components.md).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SCAN_DIRS=(
  crates/client-shared/src
  crates/client-access/src
  crates/client-connect/src
  crates/client-log/src
  crates/client-models/src
  crates/client-monitor/src
  crates/client-playground/src
  crates/client-settings/src
  crates/client-users/src
  src
)

BUTTON_RE='button[[:space:]]*\{[^}]*class:[[:space:]]*"btn-(primary|secondary|danger|ghost|black)"'
BC_DUP_RE='BCButton[^}]*class:[[:space:]]*"(btn-primary|btn-secondary|btn-danger|btn-ghost|btn-black)"'

violations=0

scan_pattern() {
  local label="$1"
  local pattern="$2"
  local hits=()

  for dir in "${SCAN_DIRS[@]}"; do
    [[ -d "$dir" ]] || continue
    while IFS= read -r -d '' file; do
      if grep -Eq "$pattern" "$file"; then
        hits+=("$file")
      fi
    done < <(find "$dir" -name '*.rs' -type f -print0)
  done

  if ((${#hits[@]} > 0)); then
    echo "::error::$label"
    printf '%s\n' "${hits[@]}"
    violations=$((violations + ${#hits[@]}))
  fi
}

scan_pattern "Raw button with btn-* class — use BCButton" "$BUTTON_RE"
scan_pattern "BCButton duplicates variant in class prop" "$BC_DUP_RE"

if ((violations > 0)); then
  echo "Found $violations UI convention violation(s). See docs/ui/components.md"
  exit 1
fi

echo "Console UI button conventions OK"
