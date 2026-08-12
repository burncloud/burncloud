---
doc_id: agent.start-here
doc_type: agent-protocol
truth: normative
status: active
---

# Start Here — AI Agent Workflow

## Goal

Reduce repository search, guessing, and accidental cross-flow regressions. Start from requested behavior, not the directory tree.

The standard loop is:

`DISCOVER -> UNDERSTAND -> TRACE -> CONTRACT -> PLAN -> CHANGE -> VERIFY -> INSPECT -> REPORT`

## 1. DISCOVER — restate the behavior

Translate the request into a user/operator-visible behavior.

Examples:

- “429 should fail over to another upstream.”
- “Admin updates a Channel.”
- “User login should reject an invalid password.”
- “Console page spacing is wrong.”

Do not begin with “edit file X” unless the user explicitly requires a file-level change.

## 2. UNDERSTAND — route the task

Open `TASK_ROUTER.md` and identify:

- primary source;
- related source;
- runtime/contract docs;
- tests/evidence to inspect;
- likely ownership domain.

If a domain contract exists, read it. Do not create a fictional domain boundary from directory names alone.

## 3. TRACE — read the real execution path

Trace only as far as required to understand the change:

`entry -> branch -> callee -> state/external effect -> return/error`

For important claims classify the edge as:

- **STATIC CONFIRMED** — directly visible in current code/tests;
- **DYNAMIC** — runtime selection/configuration/state controls the target;
- **INFERRED** — plausible but not fully proven;
- **UNKNOWN** — not yet established.

Never present DYNAMIC, INFERRED, or UNKNOWN behavior as a fixed call path.

Do not build a repository-wide call graph when a smaller trace establishes the task boundary.

## 4. CONTRACT — establish the task contract

For every non-trivial task, create the minimum internal contract defined by `TASK_CONTRACT.md`:

- goal;
- current behavior;
- expected behavior;
- scope;
- execution path;
- impacts;
- invariants;
- verification;
- done criteria.

The contract is a reasoning/control artifact. Do not commit one-off task contracts unless a long-lived repository workflow actually needs them.

## 5. PLAN — plan from root cause / behavior gap

A useful plan explains:

`problem -> root cause/behavior gap -> affected path -> required change -> verification strategy`

A list of filenames is not a sufficient plan.

## 6. CHANGE — make the smallest coherent modification

Prefer one runtime behavior per change. Avoid opportunistic refactors unless required for correctness or testability.

Follow the relevant task playbook under `playbooks/`.

## 7. VERIFY — prove function, regression, and invariants

Read `INVARIANTS.md`, `TEST_MATRIX.md`, and `verification/VERIFICATION_STANDARD.md`.

Verification answers three different questions:

1. Does the requested behavior work?
2. Did adjacent existing behavior remain intact?
3. Do the relevant BurnCloud invariants still hold?

One green test does not automatically answer all three.

## 8. INSPECT — review the final diff

Check for:

- unrelated changes;
- accidental deletion;
- public API or business-semantic drift;
- changed error/retry behavior;
- debug or temporary code;
- secrets;
- weakened tests;
- documentation that became stale.

## 9. REPORT — provide evidence

A completion report should separate:

- result;
- root cause or implementation rationale;
- verified execution path;
- changes;
- verification actually run;
- invariants checked;
- known dynamic/unverified boundaries;
- remaining risk;
- unrelated changes (normally `None`).

## Evidence references

Prefer stable references:

`path/to/file.rs :: SymbolName`  
`path/to/test.rs :: test_name`

Use line numbers only for point-in-time review evidence because they drift as code changes.
