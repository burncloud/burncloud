---
doc_id: agent.change-protocol
doc_type: agent-protocol
truth: normative
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Change Protocol

## 1. Plan from behavior

State:

- requested user/operator behavior,
- current behavior confirmed from source,
- smallest implementation boundary,
- affected state/external effects,
- tests that should prove the change.

## 2. Code

Change the smallest coherent runtime slice. Preserve unrelated behavior. Do not combine cleanup with behavior changes unless the cleanup is required for correctness.

## 3. Test

Run targeted checks first. Expand verification when the change touches shared types, route composition, database APIs, billing, authentication, or workspace dependencies.

Record what was actually run; never turn an unavailable tool into a claimed pass.

## 4. Documentation

Update a doc only if its declared truth changed. In particular update:

- `TASK_ROUTER.md` when ownership moves,
- `INVARIANTS.md` when a cross-cutting behavior changes,
- `TEST_MATRIX.md` when verification ownership changes,
- current architecture/contracts when their facts change.

## 5. Review impact

Before completion answer:

- Who calls/enters this behavior?
- What state does it read/write?
- What external requests can it make?
- What failure/retry paths changed?
- What billing/auth/routing behavior can be affected?
- What tests cover the changed branch?

## 6. Commit/PR

Use the repository's current contributor conventions visible in surrounding history/configuration. Do not create special root-level message files or other artifacts unless the repository currently requires them through tooling.

A PR description should separate:

- behavior change,
- implementation,
- verification,
- docs/contracts changed,
- known dynamic/unverified boundaries.
