---
doc_id: ui.components
doc_type: engineering-standard
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# UI Component Conventions

This page mirrors rules currently enforced by `crates/loops/src/gates/ui_conventions.rs`.

## Enforced rules

### Use `BCButton` instead of raw button + legacy `btn-*` variants

The gate rejects raw `button` usage whose class contains:

- `btn-primary`
- `btn-secondary`
- `btn-danger`
- `btn-ghost`
- `btn-black`

Use the shared `BCButton` component and its variant API instead.

### Do not duplicate a `BCButton` variant in `class`

The gate rejects a `BCButton` that also supplies one of the legacy variant classes above through its `class` prop.

## Source of truth

- Enforcement: `crates/loops/src/gates/ui_conventions.rs`
- Shared client code: `crates/client/crates/client-shared/`

Do not extend this page with preferred components unless the rule is represented by current code, a gate, or an accepted maintainer decision.
