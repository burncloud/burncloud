---
doc_id: docs.index
doc_type: agent-index
truth: normative
status: active
audited_against: 956041a8b54d8c6964e57fa2284f825cc322b0d2
---

# BurnCloud Engineering Docs

`docs/` is the engineering harness for humans and AI agents working on BurnCloud. It is intentionally code-first and should remain smaller than the implementation it explains.

The repository-level agent entrypoint is [`../AGENTS.md`](../AGENTS.md).

## Purpose

These docs exist to help an engineer or agent answer six questions quickly:

1. What user/runtime behavior does this task affect?
2. What is the real entrypoint and execution path?
3. Which code is the primary implementation?
4. Which invariants must not be broken?
5. Which tests should be run?
6. When docs and code disagree, which source wins?

## Start here

- [`../AGENTS.md`](../AGENTS.md) — repository-level agent operating contract.
- [`CLAUDE.md`](CLAUDE.md) — short bootstrap context.
- [`agent/START_HERE.md`](agent/START_HERE.md) — required task workflow.
- [`agent/TASK_ROUTER.md`](agent/TASK_ROUTER.md) — task -> runtime area -> source -> tests.
- [`agent/DOC_PRIORITY.md`](agent/DOC_PRIORITY.md) — truth hierarchy and conflict resolution.
- [`agent/INVARIANTS.md`](agent/INVARIANTS.md) — verified cross-cutting behavior.
- [`agent/TEST_MATRIX.md`](agent/TEST_MATRIX.md) — affected area -> test scope.
- [`agent/CHANGE_PROTOCOL.md`](agent/CHANGE_PROTOCOL.md) — plan/code/test/docs/commit loop.

## Current-system references

- [`architecture/CURRENT_SYSTEM.md`](architecture/CURRENT_SYSTEM.md)
- [`contracts/ROUTER.md`](contracts/ROUTER.md)
- [`standards/RUST.md`](standards/RUST.md)
- [`standards/SERVER.md`](standards/SERVER.md)
- [`standards/DATABASE.md`](standards/DATABASE.md)

## Runtime Atlas

Repository-local runtime flow documents live under [`runtime/`](runtime/README.md).

Current source-derived flow:

- [`runtime/CHAT_COMPLETIONS.md`](runtime/CHAT_COMPLETIONS.md) — `POST /v1/chat/completions` from unified server entry through auth/admission, model routing, upstream execution, billing/logging, quota settlement, and response.

Use [`runtime/FLOW_TEMPLATE.md`](runtime/FLOW_TEMPLATE.md) for future End-to-End Request Flow + progressive ICFG documents.

## Truth policy

`Source code > executable tests > current contracts/invariants > current architecture/runtime docs > engineering standards > historical/external explanatory docs`.

A document never overrides observable code behavior. If a normative rule intentionally changes desired behavior, code and tests must change in the same workstream before that rule can be treated as implemented.

Runtime documents must distinguish:

- **STATIC CONFIRMED** — proven by current source/tests;
- **DYNAMIC** — runtime data/configuration selects the branch/target;
- **INFERRED** — plausible but not fully proven.

Do not convert a dynamic or inferred edge into a fixed call graph.

## What is deliberately not stored here

- Product roadmaps or speculative future architecture mixed with current truth.
- Planned database tables mixed with current schema facts.
- Historical issue reports and one-off audit snapshots as architecture truth.
- Screenshots or other binary documentation assets.
- Duplicate constitutions that can drift apart.
- Generated function-by-function prose that is not tied to a user/runtime flow.
- Giant repository-wide call graphs that are impossible to review or keep current.

Future plans belong in GitHub Issues/Projects/PRs. The Docusaurus site at `https://burncloud.github.io/` can render human-oriented runtime documentation, but repository-local Markdown should increasingly own source-derived runtime truth so documentation can evolve with the same commit as code.

## Maintenance rule

When code changes observable runtime behavior, routing, persistence, authentication, billing, provider dispatch, failure behavior, or a listed invariant, update the relevant doc in the same PR.

Keep docs smaller than the code they point to; use file paths and symbols as evidence instead of duplicating implementation detail.
