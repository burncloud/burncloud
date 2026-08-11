---
doc_id: runtime.index
doc_type: runtime-navigation
truth: informational
status: active
audited_against: 956041a8b54d8c6964e57fa2284f825cc322b0d2
---

# Runtime Flow Navigation

Repository-local runtime documents are the versioned navigation layer from user behavior to source evidence.

Reading model:

`User Action -> End-to-End Flow -> Drill-down ICFG -> Decision/Side Effects -> Source Evidence`

The purpose is not to duplicate the source tree. A runtime document should expose the smallest execution map needed to understand and safely change a user-visible behavior.

## Available flows

| User/runtime behavior | Document | Status |
|---|---|---|
| OpenAI-compatible Chat Completions | [`CHAT_COMPLETIONS.md`](CHAT_COMPLETIONS.md) | source-derived |

Use [`FLOW_TEMPLATE.md`](FLOW_TEMPLATE.md) when adding the next flow.

## Recommended expansion order

Add flows one user journey at a time. Suggested next targets:

1. public register/login;
2. API token creation/use;
3. channel create/update/delete;
4. channel selection/scheduler/affinity drill-down;
5. provider-specific execution flows;
6. billing/quota settlement;
7. logs/monitoring;
8. streaming response handling.

Do not generate all flows in one task. Each flow should be source-audited and reviewable on its own.

## Agent usage

Before changing runtime behavior:

1. start at `docs/agent/START_HERE.md`;
2. route the task with `docs/agent/TASK_ROUTER.md`;
3. open the relevant runtime flow if one exists;
4. re-confirm every affected branch in the current source;
5. inspect executable tests;
6. classify dynamic/inferred edges honestly;
7. update the runtime doc only when the observable truth changes.

The runtime document is not authority over source code.

## Flow quality rules

A good flow:

- starts from one concrete user/operator action;
- identifies the real entrypoint;
- separates admission, routing, execution, state effects, and response behavior;
- expands important branches progressively instead of drawing a giant call graph;
- marks **STATIC CONFIRMED**, **DYNAMIC**, and **INFERRED** behavior;
- lists failure exits that have different client/state consequences;
- cites stable source symbols;
- records test/source contradictions as verification gaps instead of guessing.

## External renderer

The human-facing Docusaurus Runtime Flow & ICFG site remains available at `https://burncloud.github.io/`.

Its role should increasingly be rendering/navigation. Repository-local source-derived Markdown is the preferred owner for runtime truth because it can evolve in the same commit/PR as code.
