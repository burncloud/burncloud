---
doc_id: ui.system
doc_type: current-architecture
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Current UI Engineering Shape

BurnCloud's client is Dioxus-based and split across `crates/client` plus feature crates under `crates/client/crates/`.

For AI-agent work, the important current control points are:

- shared UI/styles: `crates/client/crates/client-shared/`;
- feature/page crates: `crates/client/crates/client-*`;
- CSS naming enforcement: `crates/loops/src/gates/css_naming.rs`;
- component convention enforcement: `crates/loops/src/gates/ui_conventions.rs`;
- shell/PowerShell checks under `crates/client/scripts/`;
- browser/aesthetic/CSS E2E coverage under `crates/tests/tests/e2e/`.

Do not infer a component or token rule from old screenshots. Check the current component implementation, current styles, and executable gate.
