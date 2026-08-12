---
doc_id: agent.task-contract
doc_type: agent-protocol
truth: normative
status: active
---

# Task Contract

A Task Contract converts a user request into an explicit engineering boundary before a non-trivial edit begins.

It exists to prevent three common agent failures:

1. editing before understanding current behavior;
2. expanding scope while solving the task;
3. declaring completion without an explicit proof target.

## When required

Use a Task Contract for any change that can affect runtime behavior, public/API behavior, persistence, routing, auth, billing, provider execution, concurrency, migrations, cross-crate APIs, or multiple files with coupled behavior.

A trivial typo or purely local documentation correction does not require a formal contract.

## Minimum template

```yaml
goal:
  Observable user/operator outcome.

current_behavior:
  What current evidence proves today.

expected_behavior:
  Observable behavior after the change.

entry:
  Route, CLI command, UI event, background trigger, or other real entrypoint.

execution_path:
  Smallest verified path relevant to the change.

scope:
  allowed:
    - Intended files/domains/components.
  avoid:
    - Boundaries that should not change unless new evidence requires it.

domains:
  - Relevant ownership domains.

impact:
  persistence: none | describe
  external_calls: none | describe
  billing_usage_quota: none | describe
  auth_authorization: none | describe
  routing_provider: none | describe
  concurrency_transactions: none | describe

invariants:
  - IDs from INVARIANTS.md, or explicit candidate invariants requiring review.

evidence:
  - path/to/file.rs :: SymbolName
  - path/to/test.rs :: test_name

verification:
  - Targeted checks.
  - Regression checks.
  - Runtime/E2E checks when needed.

done_when:
  - Observable completion criteria.
```

## Scope discipline

`allowed` is not permission to blindly edit every listed file. It describes the anticipated boundary.

`avoid` is not an absolute prohibition. If source evidence proves the root cause crosses that boundary, update the task contract before expanding the change.

## Evidence discipline

Classify material execution claims as:

- STATIC CONFIRMED;
- DYNAMIC;
- INFERRED;
- UNKNOWN;
- RUNTIME VERIFIED.

Do not list an inferred call path as evidence without labeling the uncertain edge.

## Root-cause rule

For bug fixes, the contract should contain the observed failure and the smallest evidence-backed root cause before implementation when the root cause can be established.

Do not force a false root cause when evidence is incomplete. Mark it UNKNOWN and continue investigation.

## Completion rule

The task contract is satisfied only when the final diff remains within the evidence-backed scope, required verification has been run or explicitly reported as unavailable, and the applicable Definition of Done is met.
