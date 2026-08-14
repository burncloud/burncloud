---
doc_id: ui.system
doc_type: current-architecture
truth: source-derived
status: active
audited_against: 74ee1d6212f4ab796838bbd824885a3095b7bfb9
---

# Current UI Engineering Shape

BurnCloud's current client is a Dioxus application rooted under `crates/client/src/`. The routed console uses the functional/critical page modules wired by `src/app.rs`; older feature-crate styling is not the source of truth for the rebuilt console.

Current control points:

- application routes and CSS load order: `crates/client/src/app.rs`;
- authenticated console shell: `crates/client/src/functional_layout.rs`;
- canonical semantic visual layer: `crates/client/src/visual_system.css`;
- shared legacy/base selectors: `crates/client/src/styles.css`;
- product workflow primitives: `crates/client/src/product_ui.css`;
- critical/auth page compatibility styles: `crates/client/src/critical_pages.css`;
- desktop chrome styles: `crates/client/src/desktop_chrome.css`;
- current routed pages: `crates/client/src/critical_pages/` and `crates/client/src/functional_pages/`;
- executable checks: `crates/client/scripts/check-*.sh` and `.github/workflows/client-ui.yml`.

## Cascade contract

The console intentionally keeps compatibility CSS while Phase 2 consolidates visual decisions. `visual_system.css` must remain the final CSS layer loaded by `app.rs`. It provides semantic tokens and final foundational component decisions without forcing page logic to duplicate visual values.

Page modules own information architecture and state. The visual system owns hierarchy, palette, rhythm, shape, elevation, control treatment, focus behavior and motion.

Do not infer a component or token rule from old screenshots or pre-rebuild feature crates. Inspect the current routed implementation and the canonical visual layer first.
