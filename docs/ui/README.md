---
doc_id: ui.index
doc_type: engineering-standard
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# UI Standards — Gate-Aligned Index

These files are intentionally retained because current UI/loop tooling references `docs/ui/*` paths in diagnostics and acceptance guidance.

They are not a design manifesto. They document only rules that are visible in current source/gates.

- [`components.md`](components.md) — component conventions enforced by `ui_conventions` gate.
- [`naming.md`](naming.md) — class-name rules enforced by `css_naming` gate.
- [`tokens.md`](tokens.md) — where current CSS token truth lives.
- [`system.md`](system.md) — current Dioxus/UI enforcement shape.
- [`pages.md`](pages.md) — page-change verification guidance.

Primary executable sources:

- `crates/loops/src/gates/ui_conventions.rs`
- `crates/loops/src/gates/css_naming.rs`
- `crates/client/crates/client-shared/src/styles/`
- `crates/tests/tests/e2e/`

If this documentation conflicts with an executable gate, the gate wins and the doc must be updated.
