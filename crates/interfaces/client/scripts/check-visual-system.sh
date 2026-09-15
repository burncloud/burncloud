#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VISUAL="src/visual_system.css"
APP="src/app.rs"

require() {
  local needle="$1"
  local file="$2"
  if ! grep -Fq -- "$needle" "$file"; then
    echo "Missing visual-system contract: '$needle' in $file" >&2
    exit 1
  fi
}

[[ -f "$VISUAL" ]] || {
  echo "Missing canonical visual system: $VISUAL" >&2
  exit 1
}

# The canonical layer must remain last so semantic decisions win over legacy/page CSS.
last_style=$(grep 'include_str!(".*\.css")' "$APP" | tail -n1)
if [[ "$last_style" != *'include_str!("visual_system.css")'* ]]; then
  echo "visual_system.css must be the final CSS layer loaded by app.rs" >&2
  exit 1
fi

# Token families required by the Phase 2 visual system.
for token in \
  '--bc-color-canvas' \
  '--bc-color-surface' \
  '--bc-color-text-primary' \
  '--bc-color-text-secondary' \
  '--bc-color-border' \
  '--bc-color-brand' \
  '--bc-color-success' \
  '--bc-color-warning' \
  '--bc-color-danger' \
  '--bc-space-4' \
  '--bc-radius-md' \
  '--bc-shadow-card' \
  '--bc-control-md' \
  '--bc-content-max' \
  '--bc-motion-fast'; do
  require "$token" "$VISUAL"
done

# Existing product CSS consumes these aliases; removing them causes visual drift.
for alias in '--canvas:' '--text:' '--muted:' '--border:' '--panel:' '--surface-subtle:' '--shadow-card:'; do
  require "$alias" "$VISUAL"
done

# Accessibility and interaction behavior are part of the visual system, not optional polish.
require ':focus-visible' "$VISUAL"
require '@media (prefers-reduced-motion: reduce)' "$VISUAL"
require '.button-primary' "$VISUAL"
require '.input:focus' "$VISUAL"
require '.badge-success' "$VISUAL"
require '.product-status-card.status-ready' "$VISUAL"
require '.danger-zone' "$VISUAL"

# Keep raw palette choices centralized. New page-level visual CSS should consume semantic tokens.
if grep -En -- '#[0-9A-Fa-f]{3,8}' src/product_ui.css >/dev/null; then
  echo "product_ui.css still contains raw color literals; consume semantic visual-system tokens instead" >&2
  exit 1
fi

echo "Visual system contracts OK"
