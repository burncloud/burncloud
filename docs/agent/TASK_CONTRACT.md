---
doc_id: agent.task-contract
doc_type: agent-protocol
truth: normative
status: active
---

# Task Contract

A Task Contract converts a READY engineering issue or user task into an explicit execution boundary before a non-trivial edit begins.

It exists to prevent four common agent failures:

1. editing before understanding current behavior;
2. expanding scope while solving the task;
3. inventing new abstractions instead of reusing current ones;
4. declaring completion without an explicit proof target.

A Task Contract does not grant more authority than its parent Issue. It may narrow the implementation scope after source investigation, but it must not silently widen the approved architecture boundary.

## When required

Use a Task Contract for any change that can affect runtime behavior, public/API behavior, persistence, routing, auth, billing, provider execution, concurrency, migrations, cross-crate APIs, process/runtime lifecycle, or multiple files with coupled behavior.

A trivial typo or purely local documentation correction does not require a formal contract.

If the task comes from a GitHub Issue, implementation must not begin unless that Issue is `READY` according to `ISSUE_STANDARD.md`.

## Minimum template

```yaml
issue:
  id: ISSUE-123 | none
  status: READY | direct-user-task
  plan_page: optional

goal:
  Observable user/operator outcome.

current_behavior:
  What current evidence proves today.

entry:
  Real route, CLI command, UI event, background trigger, source symbol, or other starting point.

execution_path:
  Smallest currently verified path relevant to the change.
  Mark uncertain edges INFERRED / UNKNOWN instead of inventing them.

reuse_targets:
  - Existing components/contracts that should be reused.

do_not_recreate:
  - Duplicate subsystems or sources of truth forbidden by the Issue.

expected_behavior:
  Observable behavior after the change.

behavior_contract:
  inputs:
    - Semantic inputs.
  outputs:
    - Semantic outputs.
  ownership:
    - Which component owns the behavior/state.
  side_effects:
    - none | explicit side effects.

failure_behavior:
  on_failure:
    - Explicit failure semantics.
  forbidden_fallbacks:
    - Silent fallback behaviors that would change system meaning.

scope:
  allowed:
    - Intended files/domains/components.
  avoid:
    - Boundaries that must not change without a new decision.

domains:
  - Relevant ownership domains.

impact:
  persistence: none | describe
  external_calls: none | describe
  billing_usage_quota: none | describe
  auth_authorization: none | describe
  routing_provider: none | describe
  concurrency_transactions: none | describe
  public_api_cli: none | describe
  process_runtime_lifecycle: none | describe

invariants:
  - IDs from INVARIANTS.md, or explicit candidate invariants requiring review.

dependencies:
  - Required Issue / decision / environment / test asset, or none.

evidence:
  - STATIC CONFIRMED — path/to/file.rs :: SymbolName
  - RUNTIME VERIFIED — runtime evidence

stop_conditions:
  - Conditions that require stopping instead of widening scope.

verification:
  targeted:
    - Concrete targeted checks/tests.
  regression:
    - Existing behaviors that must remain green.
  runtime_e2e:
    - Runtime/E2E checks when applicable.
  protected_behavior:
    - High-value semantics that must not be weakened.

done_when:
  - Observable completion criteria.
```

## Preflight rule

Before editing, compare the parent Issue with current `main` and answer:

1. Does the Issue evidence still hold?
2. Is the stated Entry real?
3. Do the Reuse Targets still exist and own the expected responsibility?
4. Can the relevant execution path be proven far enough to make a bounded change?
5. Are all hard dependencies actually available?
6. Is the approved scope sufficient without crossing an Avoid boundary?
7. Is any architecture/invariant change now required that the Issue did not authorize?
8. Can the required verification be meaningfully executed?

If the answer exposes a material conflict, stop before implementation.

## Scope discipline

`allowed` is not permission to blindly edit every listed file. It describes the anticipated authority boundary.

`avoid` is not an invitation to cross the boundary after discovering inconvenience. If source evidence proves the requested outcome requires an Avoid-domain change, trigger a Stop Condition and report the conflict. Update or split the Issue before continuing.

Do not use “repair by widening scope.”

## Reuse discipline

If an Issue names a Reuse Target, inspect it before creating a replacement abstraction.

A new parallel subsystem, duplicate source of truth, duplicate router/gateway/downloader/state store, or equivalent architectural fork requires explicit evidence and architecture approval. Convenience is not sufficient justification.

## Contract discipline

The Task Contract locks semantic behavior, not arbitrary implementation details.

Preserve the Issue's Inputs / Outputs / Ownership / Side Effects and Failure Behavior unless current evidence proves the Issue is invalid. In that case stop and report; do not silently rewrite the contract to match the patch.

## Evidence discipline

Classify material execution claims as:

- STATIC CONFIRMED;
- DYNAMIC;
- INFERRED;
- UNKNOWN;
- RUNTIME VERIFIED.

Do not list an inferred call path as confirmed evidence. Do not convert planning documents or comments into higher-authority runtime facts.

## Stop rule

When any Stop Condition is triggered:

```text
SCOPE / ARCHITECTURE CONFLICT DETECTED
No out-of-scope code changed.
Evidence: ...
Conflict: ...
Decision required: ...
```

The agent must not:

- widen scope on its own;
- modify unrelated modules to make tests green;
- create a duplicate subsystem to avoid an existing boundary;
- weaken tests or failure semantics to fit the implementation;
- modify architecture/invariant docs merely to legitimize the patch.

## Root-cause rule

For bug fixes, the contract should contain the observed failure and the smallest evidence-backed root cause before implementation when the root cause can be established.

Do not force a false root cause when evidence is incomplete. Mark it `UNKNOWN` and continue investigation only within the approved authority boundary.

## Verification rule

Verification must distinguish:

- **Targeted** — proves the new/change behavior;
- **Regression** — proves relevant existing behavior is preserved;
- **Runtime/E2E** — proves dynamic behavior where static tests are insufficient;
- **Protected behavior** — proves important invariant semantics were not weakened.

A patch author adding only new tests for its own new abstraction is not sufficient when existing behavior can regress.

## Completion rule

The Task Contract is satisfied only when:

1. the final diff remains within the evidence-backed authority boundary;
2. Reuse Targets were reused or an explicit approved decision explains why not;
3. Behavior Contract and Failure Behavior are preserved;
4. no Stop Condition remains unresolved;
5. required verification has been run or explicitly reported as unavailable;
6. applicable invariants and Definition of Done are satisfied;
7. the change enters `main` only through a Pull Request.
