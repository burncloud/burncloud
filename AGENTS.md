# BurnCloud Agent Instructions

This file is the repository-level constitution and router for coding agents. Keep it compact. Detailed execution knowledge belongs under `docs/agent/`, while architecture/runtime truth belongs in the existing `docs/architecture/`, `docs/contracts/`, and `docs/runtime/` trees.

## Mission

An agent working on BurnCloud must safely modify the real system, not merely generate plausible code.

Core principles:

- **Understand before change.**
- **Evidence before assumption.**
- **Invariants before implementation.**
- **Verification before completion.**

## Authority

When sources disagree, use this order:

`runtime evidence > executable tests > current source code > database schema/migrations > configuration > source-derived contracts/invariants > architecture/runtime docs > engineering standards > comments > assumptions`

A document never makes behavior true. Re-confirm the relevant source before changing it. Tests are evidence of covered behavior, not automatic proof that the test itself defines the intended contract.

## Required bootstrap

Before making a non-trivial repository change:

1. Read `docs/CLAUDE.md`.
2. Read `docs/agent/START_HERE.md`.
3. Route the behavior with `docs/agent/TASK_ROUTER.md`.
4. Create the minimum task contract described in `docs/agent/TASK_CONTRACT.md`.
5. Read the smallest real execution path required for the task.
6. Check `docs/agent/INVARIANTS.md` and `docs/agent/INVARIANT_STANDARD.md`.
7. Select verification from `docs/agent/TEST_MATRIX.md` and `docs/agent/verification/VERIFICATION_STANDARD.md`.
8. Follow `docs/agent/CHANGE_PROTOCOL.md` and the relevant playbook under `docs/agent/playbooks/`.

For data-plane behavior, also open `docs/runtime/README.md` and the relevant runtime-flow document if one exists.

## Agent constitution

### RULE-001 — Code First
Confirm current behavior from the repository and runtime evidence. Do not infer behavior from filenames, comments, old docs, or framework conventions.

### RULE-002 — No Architecture Guessing
Never replace an unverified edge with what a conventional system would probably do.

### RULE-003 — Trace Before Change
For runtime changes, trace at least `entry -> branch -> callee -> state/external effect -> return/error` before editing.

### RULE-004 — Identify the Domain
Classify the task into the smallest stable ownership domain and read its contract when one exists.

### RULE-005 — Check Invariants
Determine which verified truths must remain true. A green build does not prove semantic correctness.

### RULE-006 — Smallest Correct Change
Prefer the smallest coherent change that fixes the root cause or implements the requested behavior.

### RULE-007 — Root Cause Before Patch
Do not hide failures with default values, swallowed errors, skipped checks, or unrelated fallbacks unless that behavior is explicitly part of the contract.

### RULE-008 — Preserve Existing Behavior
Unless requested otherwise, preserve public APIs, response/error semantics, persistence, billing, routing, authentication, streaming, and provider compatibility.

### RULE-009 — No Silent Business Logic Change
Pricing, quota, usage, billing, provider/channel selection, auth, authorization, and provenance are high-risk behavior and must not change incidentally during refactors.

### RULE-010 — Do Not Hide Failure
Never make a task look green by deleting assertions, weakening tests, converting failure into success, or suppressing errors.

### RULE-011 — Tests Are Evidence, Not Absolute Truth
If a relevant test conflicts with source-derived contracts or requested behavior, investigate the contradiction before changing either side.

### RULE-012 — New Behavior Requires Verification
Every new or changed observable behavior must have a concrete verification path.

### RULE-013 — High-Risk Changes Need Regression Checks
Billing, usage, provider adapters, streaming, auth, routing, migrations, provenance, concurrency, and transaction changes require adjacent regression verification.

### RULE-014 — Never Invent APIs
Confirm SDK calls, provider parameters, internal functions, schema fields, environment variables, and configuration keys from authoritative definitions.

### RULE-015 — Respect Existing Architecture
Use established interfaces/adapters/services/repositories/registries unless changing the architecture is required and justified.

### RULE-016 — Search Is Discovery, Not Proof
A text or symbol search identifies candidates; only the verified execution path establishes runtime ownership.

### RULE-017 — Runtime Flows Must Be Traceable
Important flow nodes must map to a real route, symbol, file, configuration, schema, or dynamically selected boundary.

### RULE-018 — Distinguish Fact, Dynamic Edge, Inference, and Unknown
Use `STATIC CONFIRMED`, `DYNAMIC`, `INFERRED`, and `UNKNOWN` precisely. Never present a dynamic or inferred edge as a fixed architecture fact.

### RULE-019 — Inspect the Final Diff
Check for unrelated changes, accidental deletions, public-contract changes, secrets, debug code, generated noise, and temporary hacks.

### RULE-020 — Verification Before Completion
Do not declare completion because code was written, compilation succeeded, or one test passed. Meet the task's Definition of Done.

## Standard execution loop

`DISCOVER -> UNDERSTAND -> TRACE -> CONTRACT -> PLAN -> CHANGE -> VERIFY -> INSPECT -> REPORT`

For documentation-only work, TRACE may mean verifying the source/doc ownership being described rather than tracing a runtime request.

## Required task contract

Before editing a non-trivial behavior, establish:

- **Goal** — requested user/operator-visible outcome.
- **Current behavior** — what current evidence proves.
- **Expected behavior** — observable target state.
- **Entry** — route, CLI command, UI event, background trigger, or document ownership point.
- **Execution path** — smallest relevant path.
- **Scope** — files/domains allowed and boundaries intentionally avoided.
- **Impact** — persistence, external calls, billing, auth, routing, concurrency, tests.
- **Invariants** — truths that must remain true.
- **Verification** — checks that will prove the change.
- **Done when** — explicit completion criteria.

See `docs/agent/TASK_CONTRACT.md`.

## Documentation router

- Agent operating model: `docs/agent/README.md`
- Start workflow: `docs/agent/START_HERE.md`
- Behavior-to-source routing: `docs/agent/TASK_ROUTER.md`
- Task contract: `docs/agent/TASK_CONTRACT.md`
- Current verified invariants: `docs/agent/INVARIANTS.md`
- Invariant authoring standard: `docs/agent/INVARIANT_STANDARD.md`
- Domain contract standard: `docs/agent/domains/README.md`
- Task playbooks: `docs/agent/playbooks/`
- Test ownership: `docs/agent/TEST_MATRIX.md`
- Verification levels: `docs/agent/verification/VERIFICATION_STANDARD.md`
- Definition of Done: `docs/agent/verification/DEFINITION_OF_DONE.md`
- Change/report discipline: `docs/agent/CHANGE_PROTOCOL.md`

## Runtime documentation rule

If a change alters observable routing, authentication/admission, provider dispatch, failover, persistence, billing/accounting, response behavior, or a documented invariant, update the corresponding repository-local runtime/contract document in the same change.

Runtime documents are navigation and evidence maps, not replacements for source code. Prefer stable references such as:

`path/to/file.rs :: SymbolName`

Use line numbers only for point-in-time review evidence.

## Definition of done

A task is complete only when the applicable checklist in `docs/agent/verification/DEFINITION_OF_DONE.md` is satisfied, required checks pass (or un-runnable checks are explicitly reported), documentation reflects changed truths, and the final diff contains no unrelated changes.
