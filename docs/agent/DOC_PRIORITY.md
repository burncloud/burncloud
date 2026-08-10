---
doc_id: agent.doc-priority
doc_type: truth-policy
truth: normative
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Documentation Truth Hierarchy

## Conflict resolution

When sources disagree, use this order:

1. **Current source code** — what the program can do.
2. **Executable tests** — behavior the repository actively asserts.
3. **Current contracts/invariants** — behavior maintainers intend to preserve.
4. **Current architecture docs** — organization and ownership summaries.
5. **Engineering standards** — preferred implementation patterns.
6. **External explanatory docs** — useful navigation, not authority over source.
7. **Product planning** — future intent only; not stored in this cleaned `docs/` tree.

## Document truth values

Every maintained document should declare one of:

- `normative` — a maintainer rule or invariant.
- `source-derived` — description audited against current source.
- `generated` — machine-produced facts; do not hand edit.
- `informational` — explanatory material with no authority over source.

## Rules for agents

- A `normative` document that is not implemented does **not** make the behavior true. Report the mismatch.
- A `source-derived` document becomes stale when source behavior changes; update it with that change.
- Never merge planned and implemented states in one table without explicit per-item status.
- Never use an old audit report as evidence of current behavior.
- Never translate a normative document into a second independently maintained copy. If bilingual output is needed, generate it from one canonical source.

## Removed legacy categories

This clean docs model intentionally removes:

- roadmap/blueprint documents from the Agent truth set,
- mixed “implemented + planned” schema references,
- duplicate constitutions,
- historical issue/audit documents,
- screenshots and image assets,
- broad code summaries that can silently drift.

Planning should live in GitHub Issues/Projects/PRs where status is explicit and reviewable.
