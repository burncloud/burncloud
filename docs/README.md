---
doc_id: docs.index
doc_type: agent-index
truth: normative
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# BurnCloud Engineering Docs

`docs/` is the engineering harness for AI agents working on BurnCloud. It is intentionally small, text-only, and code-first.

## Purpose

These docs exist to help an agent answer five questions quickly:

1. What user/runtime flow does this task affect?
2. Which code is the primary implementation?
3. Which invariants must not be broken?
4. Which tests should be run?
5. When docs and code disagree, which source wins?

## Start here

- [`CLAUDE.md`](CLAUDE.md) — short bootstrap context for agents.
- [`agent/START_HERE.md`](agent/START_HERE.md) — required task workflow.
- [`agent/TASK_ROUTER.md`](agent/TASK_ROUTER.md) — task → runtime area → source → tests.
- [`agent/DOC_PRIORITY.md`](agent/DOC_PRIORITY.md) — truth hierarchy and conflict resolution.
- [`agent/INVARIANTS.md`](agent/INVARIANTS.md) — verified cross-cutting behavior.
- [`agent/TEST_MATRIX.md`](agent/TEST_MATRIX.md) — affected area → test scope.
- [`agent/CHANGE_PROTOCOL.md`](agent/CHANGE_PROTOCOL.md) — plan/code/test/docs/commit loop.

## Current-system references

- [`architecture/CURRENT_SYSTEM.md`](architecture/CURRENT_SYSTEM.md)
- [`contracts/ROUTER.md`](contracts/ROUTER.md)
- [`standards/RUST.md`](standards/RUST.md)
- [`standards/SERVER.md`](standards/SERVER.md)
- [`standards/DATABASE.md`](standards/DATABASE.md)
- [`runtime/README.md`](runtime/README.md)

## Truth policy

`Source code > executable tests > current contracts/invariants > current architecture docs > engineering standards > external/runtime explanatory docs`.

A document never overrides observable code behavior. If a normative rule intentionally changes desired behavior, the code and tests must be changed in the same workstream before the rule can be treated as implemented.

## What is deliberately not stored here

- Product roadmaps or speculative future architecture.
- Planned database tables mixed with current schema facts.
- Historical issue reports and one-off audit snapshots.
- Screenshots or other image assets.
- Duplicate Chinese/English constitutions that can drift apart.
- Generated function-by-function prose that is not compiler-backed.

Future plans belong in GitHub Issues/Projects/PRs. Runtime explanations for humans are rendered at `https://burncloud.github.io/`; the long-term direction is for source-derived runtime docs to be versioned with this repository and rendered externally.

## Maintenance rule

When code changes observable runtime behavior, routing, persistence, authentication, billing, or an invariant listed here, update the relevant doc in the same PR. Keep docs smaller than the code they point to; use file paths and symbols as evidence instead of duplicating implementation detail.
