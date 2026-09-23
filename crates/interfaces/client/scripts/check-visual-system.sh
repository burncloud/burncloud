#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

STYLE=src/shared/ui/mod.rs
SHELL=src/shared/layout/mod.rs

require() {
  local needle="$1"
  local file="$2"
  if [[ ! -f "$file" ]]; then
    echo "Missing visual-system file: $file" >&2
    exit 1
  fi
  if ! grep -Fq -- "$needle" "$file"; then
    echo "Missing visual-system contract: '$needle' in $file" >&2
    exit 1
  fi
}

# Global styles are intentionally embedded with the reusable UI primitives.
# The former standalone CSS files no longer exist after the client migration.
require 'pub fn GlobalStyle() -> Element' "$STYLE"
require 'const STYLE: &str = r#"' "$STYLE"
require ':root {' "$STYLE"
require '--sans:' "$STYLE"
require '--mono:' "$STYLE"
require '.app-shell {' "$STYLE"
require '.page-viewport {' "$STYLE"
require 'GlobalStyle {}' "$SHELL"

# Preserve focus indication, reduced-motion support, button/status primitives,
# and a responsive buyer interface across overview, marketplace, playground,
# and API-key workflows.
require ':focus-visible' "$STYLE"
require '@media (prefers-reduced-motion: reduce)' "$STYLE"
require '.button-primary {' "$STYLE"
require '.button-secondary {' "$STYLE"
require '.badge-success' "$STYLE"
require '.badge-warning' "$STYLE"
require '.badge-error' "$STYLE"
require '.conclusion-healthy' "$STYLE"
require '.conclusion-warning' "$STYLE"
require '.marketplace-grid {' "$STYLE"
require '.marketplace-drawer {' "$STYLE"
require '.playground-grid {' "$STYLE"
require '.playground-output {' "$STYLE"
require '.api-keys-panel {' "$STYLE"
require '.api-key-modal {' "$STYLE"
require '@media (max-width: 639px)' "$STYLE"
require '@media (max-width: 420px)' "$STYLE"

echo "Visual system contracts OK"
