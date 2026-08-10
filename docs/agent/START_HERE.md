---
doc_id: agent.start-here
doc_type: agent-protocol
truth: normative
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Start Here — AI Agent Workflow

## Goal

Reduce repository search, guessing, and accidental cross-flow regressions. Start from the requested behavior, not the directory tree.

## Required workflow

### 1. Restate the task as a user/runtime behavior

Examples:

- “429 should fail over to another upstream.”
- “Admin updates a Channel.”
- “User login should reject an invalid password.”
- “Console page spacing is wrong.”

Do not begin with “edit file X” unless the user explicitly requires a file-level change.

### 2. Route the task

Open [`TASK_ROUTER.md`](TASK_ROUTER.md) and identify:

- primary source,
- related source,
- runtime flow,
- relevant tests.

### 3. Read the real entry point

Trace only as far as required to understand the change:

`entry → branch → callee → state/external effect → return/error`.

Do not build a giant repository-wide call graph.

### 4. Classify every important claim

- **STATIC CONFIRMED** — directly visible in current code/tests.
- **DYNAMIC** — trait object, runtime configuration, channel type, environment, or other runtime selection controls the target.
- **INFERRED** — reasonable but not fully statically proven.

Never present DYNAMIC/INFERRED behavior as a fixed call path.

### 5. Check invariants

Read [`INVARIANTS.md`](INVARIANTS.md). If the change would alter an invariant, call that out explicitly and update the invariant only together with code/tests.

### 6. Make the smallest coherent change

Prefer one runtime behavior per change. Avoid opportunistic refactors unless required to make the behavior correct/testable.

### 7. Run the relevant verification

Use [`TEST_MATRIX.md`](TEST_MATRIX.md). At minimum run targeted checks for the affected crate/flow; broader workspace checks are required when dependency/API boundaries change.

### 8. Update docs only when the truth changed

Update docs when the code changes:

- entry/routing behavior,
- auth/admission behavior,
- persistence/state mutation,
- billing/accounting,
- provider dispatch/failover,
- a listed invariant,
- task-routing ownership.

Do not update docs for internal refactors that preserve these truths unless source locations/ownership changed materially.

## Evidence format for agent reasoning

Prefer:

`path/to/file.rs :: SymbolName`  
`path/to/test.rs :: test_name`

Use line numbers only for point-in-time review evidence; they drift as code changes.
