---
doc_id: ui.index
doc_type: engineering-standard
truth: source-derived
status: active
audited_against: c314bff9646f9113c9a58a818552fc80c77543a6
---

# UI Standards — Current Console Index

These documents describe the rebuilt Dioxus console that is currently routed from `crates/client/src/app.rs`.

- [`product-flow.md`](product-flow.md) — canonical Console responsibility flow, page ownership, scope/evidence rules, cross-page handoffs and migration order.
- [`tokens.md`](tokens.md) — canonical semantic visual-system ownership and token rules.
- [`system.md`](system.md) — current Dioxus console/CSS architecture and cascade contract.
- [`pages.md`](pages.md) — page polish and verification protocol.
- [`components.md`](components.md) — retained component conventions enforced by executable gates.
- [`naming.md`](naming.md) — retained CSS naming rules enforced by executable gates.

Primary executable sources for the current console:

- `crates/client/src/visual_system.css`
- `crates/client/src/app.rs`
- `crates/client/src/functional_layout.rs`
- `crates/client/src/critical_pages/`
- `crates/client/src/functional_pages/`
- `crates/client/scripts/check-ui-conventions.sh`
- `crates/client/scripts/check-functional-wiring.sh`
- `crates/client/scripts/check-product-ux.sh`
- `crates/client/scripts/check-visual-system.sh`
- `.github/workflows/client-ui.yml`

If documentation conflicts with the current routed source or an executable gate, source/gates win and the documentation must be corrected in the same change.
