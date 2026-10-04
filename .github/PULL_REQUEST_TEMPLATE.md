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

## Scope check (run this, do not assume it)

Paste the output of `git diff --name-only main...HEAD` and confirm every path is inside the issue's
Allowed Paths. A clean description is not evidence that the diff is clean: two branches in this
repository carried 7 and 9 files where the issue allowed 2, because they were created while `HEAD`
pointed at another feature branch.

```
<output of: git diff --name-only main...HEAD>
```

- [ ] Every path above is inside this issue's Allowed Paths.
- [ ] If the branch was created from another branch rather than from `main`, say so here, or rebuild
      it with `git cherry-pick` onto `main`.

## Review checklist

- [ ] The change does what the issue asked, and nothing else. Scope expansions are stated above with
      a reason.
- [ ] **No test asserts the current, wrong output as the expected value.** A defect gets a bug issue
      and a failing test that references it; the expected value is never the observed one. (See the
      two rules in `.github/README.md`.)
- [ ] No test was weakened, deleted, or `ignore`d to get a green run. An `#[ignore]`d test is allowed
      only when it records a known defect and names the issue in its reason.
- [ ] No `continue-on-error`, `|| true`, or swallowed error was added to hide a failure.
- [ ] Comments that promised future work were updated if this PR completes or abandons that work.
- [ ] Genuinely pre-existing failures are named as such, with evidence they exist on `main`.
- [ ] If a crate's CI coverage changed, `.github/test-plan/coverage-matrix.md` was updated.

## Reverting

<!-- How to undo this safely if it turns out wrong: revert the commit, restore a flag, re-run a
migration. Especially for anything touching releases, mirrors, credentials or money. -->
