# BurnCloud Agent Instructions

This file is the repository entrypoint for coding agents. Keep it short. Detailed rules live under `docs/`.

## Authority

When sources disagree, use this order:

`current source code > executable tests > current contracts/invariants > current architecture/runtime docs > engineering standards > historical/external docs`

A document never makes behavior true. Re-confirm the relevant code before changing it.

## Required bootstrap

Before making a repository change:

1. Read `docs/CLAUDE.md`.
2. Read `docs/agent/START_HERE.md`.
3. Route the requested behavior with `docs/agent/TASK_ROUTER.md`.
4. Read the smallest real execution path needed for the task.
5. Check `docs/agent/INVARIANTS.md`.
6. Select verification from `docs/agent/TEST_MATRIX.md`.
7. Follow `docs/agent/CHANGE_PROTOCOL.md`.

For data-plane behavior, also open `docs/runtime/README.md` and the relevant runtime flow document if one exists.

## Execution contract

For every non-trivial change, establish these facts before editing:

- **Behavior** — the user/operator-visible behavior being changed.
- **Entry** — route, CLI command, UI event, background trigger, or other real entrypoint.
- **Execution path** — `entry -> branch -> callee -> state/external effect -> return/error`.
- **Impact** — files, crates, persistence, external calls, billing/accounting, auth, routing, and tests affected.
- **Invariants** — existing truths that must remain true.
- **Acceptance** — observable conditions that prove the task is complete.

Classify important execution claims as:

- **STATIC CONFIRMED** — directly visible in current source/tests.
- **DYNAMIC** — runtime configuration, trait/adaptor/provider/channel selection, environment, or data controls the next target.
- **INFERRED** — plausible but not fully proven from the inspected path.

Never turn a DYNAMIC or INFERRED edge into a fixed architecture claim.

## Change discipline

- Prefer one runtime behavior per change.
- Make the smallest coherent change that satisfies the acceptance criteria.
- Do not perform opportunistic refactors unless they are required for correctness or testability.
- Do not infer behavior from filenames or crate names.
- Do not hide uncertainty: mark it and identify the source needed to resolve it.
- Do not declare completion while required verification is failing.

## Runtime documentation rule

If a change alters observable routing, authentication/admission, provider dispatch, failover, persistence, billing/accounting, response behavior, or a documented invariant, update the corresponding repository-local runtime/contract document in the same change.

Runtime documents are navigation and evidence maps, not replacements for source code. Prefer stable evidence references in the form:

`path/to/file.rs :: SymbolName`

Use line numbers only for point-in-time review comments.

## Definition of done

A task is complete only when:

- the intended behavior is implemented;
- affected error/failure paths were considered;
- relevant tests/checks from `docs/agent/TEST_MATRIX.md` pass, or any un-runnable check is explicitly reported;
- observable behavior changes are reflected in docs;
- the final diff contains no unrelated changes.
