<!--
Keep this template. Delete sections that genuinely do not apply, but do not delete the evidence
section: a claim without a command and its result is not evidence.
-->

## What and why

<!-- One paragraph. What changes, and which problem or issue it closes. Use "Closes #N". -->

Closes #

## Scope

| | |
| --- | --- |
| Owner / area | <!-- e.g. Commerce pricing, Identity credentials, CI --> |
| Files touched | <!-- directories or key files; keep it honest if it is wider than expected --> |
| Behaviour change | <!-- "none (types/comments only)" or describe it precisely --> |
| Data migration | <!-- none, or the migration issue --> |

## Evidence

Every check you ran, with the exact command and its result. Do not write "tests pass" without the
command; do not paste a result you did not observe.

| Command | Result |
| --- | --- |
| | |

**What the evidence does NOT cover** — required. Name the areas, platforms or cases you did not
verify, and why (no environment, needs credentials, pre-existing failure, out of scope).

- 

## CI

| Question | Answer |
| --- | --- |
| Did a workflow actually run for this PR? | <!-- yes (link) / no, because no workflow path matches --> |
| If no: why not? | <!-- a workflow filtered out by `paths` reports nothing; that is not a pass --> |
| Required checks affected? | <!-- main currently has none; note if this PR changes that --> |

## Review checklist

- [ ] The change does what the issue asked, and nothing else. Scope expansions are stated above with
      a reason.
- [ ] No test was weakened, deleted, or `ignore`d to get a green run.
- [ ] No `continue-on-error`, `|| true`, or swallowed error was added to hide a failure.
- [ ] Comments that promised future work were updated if this PR completes or abandons that work.
- [ ] Genuinely pre-existing failures are named as such, with evidence they exist on `main`.
- [ ] If a crate's CI coverage changed, `.github/test-plan/coverage-matrix.md` was updated.

## Reverting

<!-- How to undo this safely if it turns out wrong: revert the commit, restore a flag, re-run a
migration. Especially for anything touching releases, mirrors, credentials or money. -->
