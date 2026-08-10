---
doc_id: ui.tokens
doc_type: engineering-standard
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# UI Token Source of Truth

Do not maintain a second hand-written catalog of every token in `docs/`.

Current style/token truth lives under:

`crates/client/crates/client-shared/src/styles/`

Before using or adding a color, spacing, typography, radius, shadow, or layout class:

1. search the current styles directory;
2. check `docs/ui/naming.md` for gate-rejected forms;
3. reuse an existing semantic class when one exists;
4. run the current UI/CSS gates after changes.

This page intentionally avoids copying token values because copied values drift from CSS faster than source-linked guidance.
