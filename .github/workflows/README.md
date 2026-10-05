Workflow documentation moved to ../README.md.

The overview, trigger coverage, known-failing checks, conventions and the list of planned changes now live
in `.github/README.md`; a second copy here would be a second source of truth. The matrix of which crate is
verified by which job is in `../test-plan/coverage-matrix.md`.

What follows is only what that document does **not** carry: the naming scheme for the files in this
directory, and the decisions that were measured here.

## Naming

`ci-*` are checks, `cd-*` is the release, `maintenance-*` is repository upkeep. Display names use the
stable `CI / ...` form so a check can be identified on a run page without knowing the file name. The
aggregating required check, when one exists, is named `CI Required` and carries no branch name,
timestamp or matrix value.

Job ids use domain names (`contracts`, `identity`, `supply`, `commerce`, `traffic`, `platform`,
`server`, `client`, `quality`) rather than file names.

## Every check is manual; two triggers remain

The five `ci-*` workflows carry `workflow_dispatch` and nothing else. They used to run on
`pull_request` and on a `push` to `main`; that job moved to the developer's machine, where
`.github/hooks/pre-commit` runs `cargo run --quiet -- code test --staged` before the commit exists.

Two things still trigger themselves, and both are deliberate:

| Workflow | Trigger | Why it is not manual |
| --- | --- | --- |
| `maintenance-version-tag.yml` | push touching a `Cargo.toml` | A version bump *is* the release decision. It runs the workspace gate and tags only if the root version advanced **and** the gate passed |
| `maintenance-sync-gitee.yml` | push to `main` | The mirror is only useful if it tracks `main`. It runs no checks and blocks nothing |

`cd-release.yml` is unchanged: tags, or manual. It is what the tag from `maintenance-version-tag.yml`
starts, which is why the gate has to run *before* the tag exists rather than alongside it. A separate
validation workflow on `push` would have left the tag published by the time the checks failed.

### Why the checks are defined in one file

`ci-quality.yml` names the four gate commands — `fmt --all -- --check`,
`test --workspace --no-default-features`, `clippy --workspace --all-targets --no-default-features` and
`deny check` — and it is the only file that does. `maintenance-version-tag.yml` reaches them through
`uses: ./.github/workflows/ci-quality.yml`, so the release gate cannot drift from the local gate. A
second copy of the commands would be a second thing to update and two verdicts to reconcile.

`--no-default-features` is not cosmetic and is copied from `crates/platform/code/src/plan.rs`. The
client crate's default `desktop` feature pulls GTK native libraries, so a default-feature workspace
build does not compile on a headless Ubuntu runner. `ci-client.yml` keeps the desktop build covered on
Windows and macOS, and `ci-tests.yml` takes an input that turns the flag off.

Clippy is the one command where the gate and the local hook are **not** the same, and this is measured
rather than assumed. PR #719 added `-- -D warnings` to the hook's Clippy invocation, which lints only
the packages a change affects. The same flag across `--workspace` fails today:

```
$ cargo clippy --workspace --all-targets --no-default-features -- -D warnings
error: could not compile `burncloud-code` (lib) due to 3 previous errors
error: could not compile `burncloud-loops` (lib) due to 20 previous errors
$ echo $LASTEXITCODE
101
```

So the gate runs Clippy without `-D warnings`. The consequence to keep in mind: **a change can pass the
hook and still add a warning to a crate the hook did not select.** Closing that gap means clearing the
workspace warning baseline, which is a separate change.

## Decisions measured here, not assumed

### The workspace build is green, so the gate can be workspace-wide

The previous version of this directory's documentation stated that three test targets failed to
compile and that nineteen crates' tests were run by no job. Both claims were re-measured rather than
inherited:

```
$ cargo check --workspace --all-targets --no-default-features
    Finished `dev` profile ... in 31.44s
$ echo $LASTEXITCODE
0
```

That is what makes one workspace run a replacement for five hand-maintained per-suite jobs. The
per-suite jobs named thirteen crates; `--workspace` cannot fall out of date the way a crate list can.

Careful with `cargo check` in PowerShell: the pipeline can report exit code 1 while cargo printed
`Finished`, because a native command writing to stderr is turned into an error record. Read
`$LASTEXITCODE`, or redirect stderr, before concluding a build failed.

### The gate was run before anything depended on it

Adding a workspace-wide check and *then* discovering it is red is how a gate becomes something people
ignore. The suite was run first, and it found two faults:

* `burncloud-router` `health_probe::tests::test_probe_state_management` asserted that a **Closed**
  breaker should be probed, contradicting the guard at the top of `should_probe`. Deterministic, not
  flaky: it failed on every run. Fixed in the test.
* `burncloud-service-user` `test_login_user_success` removes its temporary SQLite file 200 ms after
  closing the pool, and Windows has not always released the handle by then (`os error 32`). Observed
  once in a full-suite run and never in isolation (4/4 isolated runs passed). Left open and recorded
  in `../README.md`; treat a `service-user` failure in the gate as flaky and re-run it.

### What `actionlint` cannot check here

`actionlint` checks syntax, expressions and deprecated action versions. It cannot check the trigger
decision itself, required-check aggregation, branch protection, `merge_group` semantics, events caused
by `GITHUB_TOKEN`, the runner image, caching, or secrets. For a workflow change the authoritative check
is a real run.

It was run on these files (`actionlint 1.7.12`) and they are clean. One caveat it reports: it reads
every quoted entry under a `paths:` key, so a tag filter such as `v*` in `cd-release.yml` is reported
as matching no tracked file. That is a limitation of the check, not a defect in the workflow.

## Dependency reproducibility: `Cargo.lock` is not tracked

Stated here because a CI document that omits it invites the claim that builds are reproducible, and they
are not.

Measured:

```
$ git ls-files Cargo.lock                       -> (no output: not tracked)
$ grep -n Cargo.lock .gitignore                 -> 29:Cargo.lock
$ grep -n -- --locked .github/workflows/*.yml   -> (no matches)
```

Consequences, all of which follow from that one fact:

- **Dependency versions drift.** Cargo resolves newest-compatible versions at build time, so two runs of
  the same commit can build against different dependency versions. A build that worked can break with no
  change to this repository.
- **`--locked` cannot be used**, which is why no workflow passes it: the flag requires the file.
- **`cargo-deny` and `clippy` results are not pinned to a dependency set** either.

**The decision, recorded rather than left implicit.** Committing `Cargo.lock` is the conventional choice
for a workspace that ships binaries, and it was considered. It is **not done here** for two reasons:

1. It requires editing `.gitignore`, which the project's instructions list as a file not to modify
   without an explicit decision. Removing one line is a small change for whoever owns that file.
2. It changes what every future build resolves -- repository-wide behaviour that deserves its own change
   and its own verification rather than being folded into a CI document.

**What would be verified before committing it:**

- `cargo build --workspace --locked` succeeds from a clean checkout on the committed file.
- `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked` behave as they
  do without the flag.
- The file is regenerated whenever a manifest changes. Otherwise the next run fails with a lock-file
  mismatch rather than a code error -- worth naming, because that is the cost of the choice.

Until then, no part of this project should claim reproducible builds or pinned dependency versions.
