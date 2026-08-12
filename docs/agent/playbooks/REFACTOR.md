---
doc_id: agent.playbook.refactor
doc_type: agent-playbook
truth: normative
status: active
---

# Refactor Playbook

A refactor changes implementation structure while intentionally preserving defined behavior.

Core contract:

`observable behavior before == observable behavior after`

If observable behavior is intentionally changing, the task is not a pure refactor and must include that behavior change explicitly in the Task Contract.

## Before editing

Define the behavior that must not change:

- public interfaces;
- success/error semantics;
- persistence/state transitions;
- billing/usage/quota semantics;
- auth/authorization boundaries;
- routing/provider selection behavior;
- streaming/retry behavior;
- concurrency/transaction behavior where relevant.

Find the tests/evidence that currently protect those truths.

## Workflow

```text
DEFINE PRESERVED BEHAVIOR
 -> TRACE CURRENT OWNERSHIP
 -> DEFINE REFACTOR BOUNDARY
 -> CHANGE STRUCTURE
 -> VERIFY PRESERVED BEHAVIOR
 -> RUN RELEVANT REGRESSION
 -> INSPECT DIFF
```

## Guardrails

- Do not combine unrelated cleanup with the refactor.
- Do not silently alter business logic to make the new structure easier.
- Do not rewrite tests merely to match the new implementation shape if externally relevant behavior should remain identical.
- Update runtime/domain docs only when ownership/source locations materially change; do not churn docs for internal movement that preserves all documented truths.
