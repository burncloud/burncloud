---
doc_id: agent.playbook.bug-fix
doc_type: agent-playbook
truth: normative
status: active
---

# Bug Fix Playbook

Use this for incorrect existing behavior.

## Workflow

```text
REPRODUCE / ESTABLISH FAILURE
        -> LOCATE ENTRY
        -> TRACE EXECUTION
        -> IDENTIFY ROOT CAUSE
        -> IDENTIFY AFFECTED INVARIANTS
        -> DESIGN SMALLEST FIX
        -> IMPLEMENT
        -> TARGETED VERIFY
        -> REGRESSION VERIFY
        -> DIFF INSPECTION
        -> INVARIANT CHECK
        -> EVIDENCE REPORT
```

## Required questions

Before editing, answer:

- What exact observable behavior is wrong?
- Is the failure reproducible, test-proven, source-proven, or only reported?
- What real entry point reaches the failing branch?
- What is the smallest evidence-backed root cause?
- Which adjacent success/error/retry/streaming paths can the fix affect?
- Which invariants apply?

## Root-cause discipline

Do not treat these as default fixes:

- swallowing an error;
- inserting a zero/default value;
- skipping validation;
- disabling a test;
- adding an unrelated retry/fallback;
- catching a panic without fixing the invalid state.

They are valid only when the intended contract explicitly requires that behavior.

## Verification

At minimum:

1. prove the failing branch is corrected;
2. run/inspect the closest regression tests;
3. verify affected error/failure paths;
4. verify relevant invariants;
5. inspect the final diff for unrelated changes.

For high-risk areas, raise the verification level according to `../verification/VERIFICATION_STANDARD.md` and `../TEST_MATRIX.md`.

## Evidence report

Report:

- failure/root cause;
- verified execution path;
- changed symbols/files;
- verification actually run;
- invariants checked;
- remaining dynamic/unverified boundaries.
