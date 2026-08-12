---
doc_id: agent.definition-of-done
doc_type: verification-guide
truth: normative
status: active
---

# Definition of Done

An agent may claim a BurnCloud task is complete only when every applicable item below is satisfied or explicitly reported as not applicable/unavailable.

## Understanding

- [ ] The user/operator goal is explicit.
- [ ] Current behavior was established from appropriate evidence.
- [ ] The real entry/ownership point was found.
- [ ] The relevant execution path was traced far enough to bound the change.
- [ ] Important claims are labeled STATIC CONFIRMED, DYNAMIC, INFERRED, UNKNOWN, or RUNTIME VERIFIED as appropriate.

## Contract and scope

- [ ] The relevant domain/ownership boundary is known.
- [ ] Applicable invariants were identified.
- [ ] Root cause or implementation path is evidence-backed.
- [ ] The final change remains within the Task Contract or the contract was explicitly expanded based on evidence.
- [ ] No unrelated refactor/cleanup is mixed into the task.

## Implementation

- [ ] Requested behavior is implemented.
- [ ] Relevant failure/error/retry/streaming paths were considered.
- [ ] Public/API/persistence/billing/auth/routing semantics were not changed unintentionally.
- [ ] No debug code, secrets, disabled checks, or temporary hacks remain.

## Verification

- [ ] The requested behavior has concrete evidence.
- [ ] Required targeted checks were run or explicitly reported unavailable.
- [ ] Required regression checks were run or explicitly reported unavailable.
- [ ] Relevant invariants were re-checked.
- [ ] Verification claims describe what actually ran, not what should probably pass.

## Documentation

- [ ] Runtime/architecture/contracts were updated when their declared truth changed.
- [ ] `TASK_ROUTER.md` was updated if stable ownership moved.
- [ ] `INVARIANTS.md` was updated if an invariant intentionally changed.
- [ ] `TEST_MATRIX.md` was updated if verification ownership changed.
- [ ] No documentation was changed merely to make the implementation appear correct.

## Final inspection

- [ ] The complete diff was inspected.
- [ ] No unrelated files changed.
- [ ] No accidental deletion or generated noise is present.
- [ ] Remaining unknown/dynamic boundaries and risks are disclosed.

## Completion report

The final report should contain, when applicable:

```text
Result
Root Cause / Rationale
Verified Runtime or Ownership Path
Changes
Verification
Invariants
Evidence
Remaining Risk
Unrelated Changes
```

If a required verification step cannot run, the task can still be presented for review, but it must not be described as fully verified.
