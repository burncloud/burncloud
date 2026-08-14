---
doc_id: ui.pages
doc_type: verification-guide
truth: source-derived
status: active
audited_against: 74ee1d6212f4ab796838bbd824885a3095b7bfb9
---

# UI Page Change Protocol

For a current Console/client page change:

1. locate the routed page under `crates/client/src/critical_pages/` or `crates/client/src/functional_pages/`;
2. inspect `src/visual_system.css`, shared components and existing structural selectors before introducing a new visual primitive;
3. keep product decisions in the page and visual decisions in semantic `--bc-*` tokens/foundational rules;
4. avoid raw palette literals in page/product CSS; add or reuse a semantic token instead;
5. preserve explicit loading, empty, ready, warning, error and dangerous-operation states where the workflow supports them;
6. run `scripts/check-ui-conventions.sh`, `scripts/check-functional-wiring.sh`, `scripts/check-product-ux.sh` and `scripts/check-visual-system.sh` from `crates/client`;
7. run the affected Rust checks and relevant E2E/visual coverage when available;
8. inspect the result at the primary desktop target (1440×900) and at a narrower desktop width before treating pixel polish as complete.

## Page polish order

When refining an existing page, review it in this order:

1. **Visual hierarchy** — can the user identify the conclusion, current state and next action immediately?
2. **State and interaction** — are loading, empty, disabled, success, warning, error, confirmation and danger states explicit and consistent?
3. **Pixel polish** — spacing, alignment, typography, icon sizing, table density, borders, shadows and hover/focus behavior.

Do not use screenshots under `docs/` as acceptance truth. Visual evidence belongs in test artifacts or PR discussion; stable rules belong in source and executable gates.
