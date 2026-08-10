---
doc_id: ui.pages
doc_type: verification-guide
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# UI Page Change Protocol

For a Console/client page change:

1. locate the owning feature crate under `crates/client/crates/` or page under `crates/client/src/`;
2. inspect shared components/styles before introducing new UI primitives;
3. satisfy `ui_conventions` and `css_naming` gates;
4. run the affected client crate checks;
5. inspect/run relevant E2E coverage such as `console_pages.rs`, `css_visual_acceptance.rs`, `aesthetic_acceptance.rs`, or the specific user-flow test.

Do not use screenshots under `docs/` as acceptance truth. Visual evidence belongs to test artifacts/PR discussion; stable UI rules belong in executable gates and source.
