# BurnCloud CI

This directory holds the GitHub Actions workflows, the pull-request template, and the CI overview for
this repository. It replaces the previous `workflows/README.md`, whose entire content was "@ readme".

Status of this document: it describes **what the workflows actually do today**, verified against the
files in `workflows/`. Where something is planned but not implemented, it says so explicitly instead
of describing the intention as if it were the behaviour.

## Workflows

### Local pre-commit setup

Run `cargo run -- code init` once per checkout to install the local Git hook.
The versioned `.github/scripts/pre-commit-checks.sh` gates commits on formatting,
workspace tests, Clippy and the full `cargo deny check` policy. Existing failures
are not suppressed. See the root README for prerequisites, staging requirements
and existing hook handling. The `code-init` job in `arch.yml` tests the installer
and commit failure propagation on Windows and Linux independently of application dependencies; Cargo
is stubbed in those hook tests, so a green job is not a workspace-health result.
Python is needed only for those regression tests, not for setup or commits.

| File | Trigger | What it checks |
| --- | --- | --- |
| `arch.yml` | changes to `crates/traffic/router/Cargo.toml` or `deny.toml` | Router dependency whitelist (`crates/traffic/router/scripts/check-router-deps.sh --ci`) and `cargo-deny` bans |
| `client-ui.yml` | root `Cargo.toml`, `crates/interfaces/{cli,client}/**`, `deploy/Dockerfile`, itself | UI convention scripts, LiveView feature check, desktop builds on Windows and macOS |
| `security-billing-invariants.yml` | see "Trigger coverage" below | Five independent jobs: formatting, Node invariants, Billing invariants, Security invariants, migration contracts |
| `version-check.yml` | root `Cargo.toml`, any `crates/**/Cargo.toml`, itself; or manual | Tags the root package version when it advances |
| `release.yml` | tags | Builds release artifacts |
| `sync-to-gitee.yml` | pushes | Mirrors the repository to Gitee |
| `workflows/README.md` | — | Superseded by this file; kept only until the rename in "Planned changes" happens |

### `security-billing-invariants.yml`

Split into five jobs so that one class of failure cannot hide another. Before the split, a single
job ran everything in sequence, and the first non-zero exit ended it: the known-failing Node P0
compile error (`interfaces/server/src/node_orchestrator.rs`, E0308) terminated the job **before** the
Billing and Security suites ran, so neither had any CI coverage.

```
formatting            rustfmt on changed files; the only gate
node-invariants       cargo check, node runtime tests, interrupted-preparation, node_test
billing-invariants    billing_invariants, quota_tests, Commerce accounting tests
security-invariants   security_invariants
migration-contracts   contract golden tests, purity guards, surface guards, real-SQLite tests
```

Only `formatting` gates the rest, and **nothing depends on `node-invariants`**.

## Trigger coverage

A workflow only runs for a pull request when a changed path matches its `paths` filter. Because the
crate layout is nested (`crates/<domain>/<name>/`), a single-segment glob is easy to get wrong:

* `crates/*/Cargo.toml` matches **zero** of the 37 tracked manifests. `version-check.yml` used that
  pattern, so a crate manifest change never triggered it. Fixed to `crates/**/Cargo.toml`.
* `arch.yml` watches only two paths. A dependency change in `crates/traffic/router/Cargo.toml` is
  covered; a change in a transitive manifest is not.

`security-billing-invariants.yml` covers, in addition to its original paths:

```
crates/commerce/contracts/**     crates/supply/contracts/**     crates/traffic/contracts/**
crates/interfaces/common/**      crates/commerce/database-billing/**
crates/supply/database-channel/**  crates/identity/database-user/**
```

Areas that still have **no** CI coverage (see `test-plan/coverage-matrix.md` for the full list):
`crates/identity/service-user`, `crates/trust/inference`, `crates/platform/storage/database-sys`,
`crates/platform/configuration/setting`, `crates/platform/lifecycle/**`,
`crates/platform/storage/download**`, `crates/platform/observability/**`, `crates/traffic/router-aws`,
`crates/interfaces/service`, `crates/supply/service-channel`, `crates/supply/service-models`.

## Reproducibility limits

* **`Cargo.lock` is not tracked** (`.gitignore` line 29). CI therefore resolves dependencies fresh on
  every run, and `--locked` cannot be used. Two runs of the same commit can in principle resolve
  differently. Committing the lock file is a prerequisite for any `--locked` command; it is **not**
  done here because `.gitignore` is off limits for automated changes in this repository.
* The Rust toolchain is `stable` via `dtolnay/rust-toolchain`, so the compiler version drifts with
  upstream releases. No version is pinned.

## Known-failing checks

| Check | Cause | Status |
| --- | --- | --- |
| `node-invariants` | `interfaces/server/src/node_orchestrator.rs:855,911` E0308 (mismatched types against `platform/node/src/preparation/prepared_artifact.rs:16`) | **Fails on `main`.** Kept visible on purpose: it now reports only the Node side, and no longer hides Billing/Security |
| `security-invariants` | — | passes |
| `release.yml` action pin | `softprops/action-gh-release@v1` was pinned, which `actionlint` reports as a runner version GitHub no longer supports | **Fixed in this PR** (bumped to `@v2`); needs a release-correctness check before it is relied on. Tracked separately so it is not buried here |
| `billing-invariants` | — | passes |

Pre-existing compile failures elsewhere in the workspace, which is why no workflow runs
`cargo test --workspace`:

* `burncloud-service-inference` `tests/integration_test.rs`: `RouterDatabase::get_upstream` missing (E0599) + E0282
* `burncloud-server` `tests/log_api_tests.rs`: E0433 (`test_utils`)
* `burncloud-service-models` example: `unresolved import ...::ModelInfo` (E0432)

A workspace-wide gate would be born red, and a permanently red check is worse than no check because
it trains reviewers to ignore it.

## Branch protection

`main` currently has **no required status checks** (the only ruleset is a disabled "禁止删除main
branch"). Nothing here is enforced by the GitHub side yet. Enabling required checks is the last step
of the plan, not a current fact.

## Conventions

* Workflow files are lowercase kebab-case `.yml`. `ci-*` = checks, `cd-*` = release,
  `maintenance-*` = repository upkeep.
* Job ids use domain names (`contracts`, `identity`, `supply`, `commerce`, `traffic`, `platform`,
  `server`, `client`).
* CI orchestration lives in YAML. There is no Python executor or custom config parser: package and
  target groupings belong in the workflow matrix, and the coverage matrix is documentation only.
* `run` steps use Bash. Shell scripts must not swallow failures with `|| true`, and a step must not
  report success when its command did not run.

## Planned changes (not implemented)

Taken from `todos_1.txt` §2 and §9. Listed so the intent is on record, not to describe today's state.

```
arch.yml                       -> ci-architecture.yml
client-ui.yml                  -> ci-client.yml
security-billing-invariants.yml -> ci-tests.yml
release.yml                    -> cd-release.yml
version-check.yml              -> maintenance-version-tag.yml
sync-to-gitee.yml              -> maintenance-sync-gitee.yml
workflows/README.md            -> this file
```

Plus: a unified `ci.yml` entry point with a `CI Required` aggregation job, `merge_group` support,
and `actionlint` on the workflow files themselves. Two behaviours are deliberately **not** changed
here because getting them wrong would publish releases or break mirrors:

* A tag pushed by `version-check.yml` uses the default `GITHUB_TOKEN`, and events caused by that
  token **do not trigger further workflows** — so a tag created here may not start the release
  pipeline. The fix (a PAT, or `release.yml` listening on `workflow_run`) needs a release-correctness
  test before it lands. The workflow summary now states this so a silent no-release is visible.
* `sync-to-gitee.yml` uses `--all --force`; whether the mirror should be main-only or all refs, and
  how deletions are handled, needs verification against a temporary bare repository rather than
  trial-and-error against the real mirror.

## Two rules that are not optional

Both were learned by getting them wrong. They apply to every PR in this repository, not only to
CI or test work.

### 1. Check the actual diff before opening the PR

A clean PR description is not evidence that the diff is clean. Run:

```bash
git diff --name-only main...HEAD
```

and check every path against the Allowed Paths of the issue. This failed twice: two branches were
created while `HEAD` still pointed at another feature branch, so their diffs contained 7 and 9
files where the issue allowed 2 -- a closed PR's `.github` documentation and a sibling test PR's
changes both rode along. Both were rebuilt from `main` with `git cherry-pick`.

Why it matters more than tidiness: a reviewer cannot assess a change whose diff contains three
unrelated responsibilities, and a revert of one takes the others with it.

### 2. A test must never freeze a defect as the contract

When a test reveals that the implementation is wrong, the expected value is **not** the current
output. Writing

```rust
let observed = calculate_cost_safe(...);
assert_eq!(observed, true_value as i64); // observed is the wrapped, wrong value
```

declares a wrong result to be the contract. The practical harm is concrete: the eventual fix turns
that test red, so the fix looks like a regression and the next person either reverts it or edits the
expectation again.

The procedure instead:

```text
a test reveals wrong behaviour
        |
        v
file a bug issue stating the decision to be made
        |
        v
leave a failing test that records the defect and references the issue
  (#[ignore] with the reason, so the suite stays green and the defect stays visible)
        |
        v
decide the correct behaviour, then fix
        |
        v
remove the ignore; the same test becomes the regression test
```

A defect may be asserted as **not** what it should be (`assert_ne!` against the clamp that would be
wrong to assume), but never as the expected value. The same rule covers the neighbouring temptation:
do not `ignore` a failing test to make a suite green, and do not widen a tolerance until it passes.

Recorded because it happened: `calculate_cost_safe` wraps an out-of-range cost to
`3875820019684212736` instead of erroring, and the first version of its test asserted that value as
correct. See #640 for the defect and the decision it needs.
## Adding a workflow

1. Read `test-plan/coverage-matrix.md` to see whether the area already has a job, and extend it
   rather than adding a parallel one.
2. Keep `permissions` minimal (`contents: read` for anything that only reads).
3. Give every job a `timeout-minutes`.
4. Do not add `continue-on-error` to required tests.
5. After opening the PR, confirm the run actually happened: a workflow filtered out by `paths`
   reports nothing at all, which looks like success in the PR but proves nothing.
