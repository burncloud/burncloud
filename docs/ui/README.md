---
doc_id: ui.index
doc_type: engineering-standard
truth: source-derived
status: active
audited_against: c314bff9646f9113c9a58a818552fc80c77543a6
---

# UI Standards — Current Console Index

These documents describe the rebuilt Dioxus console that is currently routed from `crates/client/src/app.rs` and the target product semantics that future Console work must converge on.

- [`product-text-prototype.md`](product-text-prototype.md) — trust-first target product text prototype: north star, proof vocabulary, Trust Receipt, Verification and target information architecture.
- [`verification-profiles.md`](verification-profiles.md) — machine-testable proof dimensions, named verification profiles, downgrade/failure rules and permitted `CHAIN VERIFIED` wording.
- [`product-todo.md`](product-todo.md) — Goal → TODO → Verification execution plan with audited baseline, acceptance criteria and completion gates.
- [`product-flow.md`](product-flow.md) — canonical current Console responsibility flow, page ownership, scope/evidence rules, cross-page handoffs and migration order.
- [`tokens.md`](tokens.md) — canonical semantic visual-system ownership and token rules.
- [`system.md`](system.md) — current Dioxus console/CSS architecture and cascade contract.
- [`pages.md`](pages.md) — page polish and verification protocol.
- [`components.md`](components.md) — retained component conventions enforced by executable gates.
- [`naming.md`](naming.md) — retained CSS naming rules enforced by executable gates.

## Product decision precedence

For future UI work:

1. `product-text-prototype.md` defines the target product claims and trust semantics;
2. `verification-profiles.md` defines what verification labels mean and the proof required to use them;
3. `product-todo.md` defines what must be implemented and how completion is verified;
4. `product-flow.md` defines current page ownership and migration boundaries;
5. routed source and executable gates define what the current product actually does today.

Target-state documentation must never be presented as current implementation truth. If routed source lacks a target proof, the UI must show the current limitation rather than the desired future state.

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

If documentation conflicts with the current routed source or an executable gate about **current behavior**, source/gates win and the documentation must be corrected. If implementation conflicts with the agreed target product semantics, open/continue the corresponding TODO instead of rewriting the target to excuse the implementation.
