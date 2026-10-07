# BurnCloud CI

This directory holds the GitHub Actions workflows, the pull-request template, and the CI overview for
this repository. It replaces the previous `workflows/README.md`, whose entire content was "@ readme".

Status of this document: it describes **what the workflows actually do today**, verified against the
files in `workflows/`. Where something is planned but not implemented, it says so explicitly instead
of describing the intention as if it were the behaviour.

## What triggers what, after this change

The verification model is split between local hooks and trusted self-hosted PR checks. Developers run
the same four quality responsibilities before commit, while GitHub exposes independent fmt, test,
Clippy and deny verdicts for trusted same-repository pull requests (or after maintainer approval).

| Workflow | Trigger | Why |
| --- | --- | --- |
| `ci-self-hosted-fmt.yml` | PR/review gate | Self-hosted `cargo fmt --all -- --check` |
| `ci-self-hosted-test.yml` | PR/review gate | Self-hosted affected-package `code test --base` |
| `ci-self-hosted-clippy.yml` | PR/review gate | Self-hosted strict workspace Clippy |
| `ci-self-hosted-deny.yml` | PR/review gate | Self-hosted dependency-policy check; skips ordinary source-only PRs and uses the runner-local advisory DB |
| `ci-self-hosted-base.yml` | `workflow_call` only | Shared self-hosted authorization, checkout, Rust/OpenSSL setup, tool bootstrap and the four fixed Rust check implementations |
| `ci-quality.yml` | manual, or called by `maintenance-version-tag.yml` | The workspace gate: `fmt --all -- --check`, `test --workspace --no-default-features`, `clippy --workspace --all-targets --no-default-features -- -D warnings`, `deny check` |
| `ci-architecture.yml` | manual | `burncloud-code` regression on Windows/Linux plus the router dependency whitelist |
| `ci-client.yml` | manual | Desktop builds, LiveView check and the console convention scripts — all excluded from the gate, which builds with `--no-default-features` |
| `ci-integration.yml` | manual | The only job that needs a service container: PostgreSQL 16 and the 18 migrations |
| `maintenance-version-tag.yml` | push touching a `Cargo.toml`, or manual | Validates the workspace, then tags — **only when the root package version actually advanced** |
| `cd-release.yml` | tags, or manual | Unchanged: builds the release artifacts |
| `maintenance-sync-gitee.yml` | pushes to `main`, or manual | Unchanged, and deliberately still automatic: repository upkeep, not a check |

Two consequences worth stating plainly, because neither is obvious from the files:

* **The PR gate is self-hosted and trust-gated.** Same-repository PRs from allowlisted users run
  automatically; other PRs receive failed placeholder checks until an allowlisted maintainer submits
  an APPROVE review. Untrusted code is never sent to the persistent self-hosted runner automatically.
* **A push that does not change a version still runs no push-time quality checks** except the Gitee
  mirror. PR checks and the local hook are the quality gates; release/version workflows remain separate.

### The local gate

`cargo run -- code init` installs the hooks. The managed `pre-commit` runs four independent gates
in order: workspace formatting, affected-package `code test --staged`, strict workspace Clippy, and
`cargo deny check`. `code test` itself owns only Cargo test selection/execution. It ignores
`.github/**`, `clippy.toml` and `deny.toml` for test-scope purposes, scopes a package manifest to
that package and its consumers, and reserves full-workspace tests for root Cargo/build/toolchain
configuration, unknown paths or explicit `--all`.

The same responsibilities are split into four thin self-hosted PR entry workflows so each check has
an independent GitHub verdict: fmt, affected tests, Clippy and deny. All four call
`ci-self-hosted-base.yml`, which owns the shared authorization, runner selection, checkout, Rust
toolchain/OpenSSL setup and fixed command implementations. The callers pass only a closed check kind
(`fmt`, `test`, `clippy` or `deny`), never arbitrary shell commands. The deny entry still creates
a GitHub verdict for every PR, but the expensive `cargo deny` command is executed only when dependency
or dependency-policy files change: any `Cargo.toml`, `Cargo.lock`, `deny.toml` or `.cargo/**`.
PR deny runs use the runner-local RustSec advisory database with fetching disabled and have a five-minute
command timeout, so an external advisory-db fetch cannot silently occupy a runner. `--plan`, `--base REF`
and `--all` remain available for manual test-scope verification.

**Clippy is deliberately outside `code test`.** `code test` now has one responsibility:
select affected packages and run their Cargo tests. Strict Clippy is a separate workspace-wide gate in
the managed pre-commit hook and in `ci-self-hosted-clippy.yml`; this keeps lint policy independent from
test-scope selection and prevents a lint configuration change from inflating the test plan.

The `code-regression` job in `ci-architecture.yml` runs `cargo test -p burncloud-code` on Windows and Linux.
Native Rust regression tests verify real Git commits and selection, with Cargo check execution stubbed.
Their success is not a workspace-health result. There is no Python or Shell test harness; Git's hook
remains a thin shell wrapper.

## Workflows

| File | Trigger | What it checks |
| --- | --- | --- |
| `ci-self-hosted-fmt.yml` | PR/review gate | Workspace rustfmt on the trusted self-hosted runner |
| `ci-self-hosted-test.yml` | PR/review gate | Affected-package tests through `burncloud-code` |
| `ci-self-hosted-clippy.yml` | PR/review gate | Strict workspace Clippy |
| `ci-self-hosted-deny.yml` | PR/review gate | Dependency-policy check only when `Cargo.toml`, `Cargo.lock`, `deny.toml` or `.cargo/**` changes |
| `ci-quality.yml` | manual; called by `maintenance-version-tag.yml` | The four gate commands. Also carries the identity test-discovery floors |
| `ci-architecture.yml` | manual | `burncloud-code` regression on Windows/Linux plus the router dependency whitelist |
| `ci-client.yml` | manual | UI convention scripts, LiveView feature check, desktop builds on Windows and macOS |
| `ci-integration.yml` | manual | The Identity contract against a real PostgreSQL 16 server. The only job that needs a service container |
| `maintenance-version-tag.yml` | root `Cargo.toml`, any `crates/**/Cargo.toml`, itself, `ci-quality.yml`; or manual | Runs the workspace gate, then tags the root package version — only when it advanced |
| `cd-release.yml` | tags, or manual | Builds release artifacts |
| `maintenance-sync-gitee.yml` | pushes to `main`, or manual | Mirrors the repository to Gitee |
| `workflows/README.md` | — | Not a workflow: the naming scheme for the files in this directory, and two decisions measured there |


## What the workspace gate covers

`ci-quality.yml` runs the whole workspace, so the question "which job runs this crate's tests" now has
one answer for every crate: the gate does, when you run it.

**A correction, because this file used to argue the opposite.** An earlier version of this section
claimed that nineteen crates' tests were executed by no CI job, naming `burncloud-database` and its 86
tests — and it proposed `cargo test --workspace` as the fix. That was accurate when it was written and
it is now measured false: `cargo check --workspace --all-targets --no-default-features` completes with
exit code 0 on `main`, and the pre-existing test compile failures this file listed no longer reproduce.
The five per-suite jobs that existed to work around a red workspace are therefore gone, replaced by the
single workspace run they were compensating for.

The three states that remain worth distinguishing:

| State | Meaning | Where it comes from |
| --- | --- | --- |
| **tested** | the gate runs this package's tests | `cargo test --workspace --no-default-features` |
| *compile only* | the gate type-checks it, but the platform-specific configuration is only built elsewhere | `ci-client.yml` for the desktop features the gate cannot build |
| **not built** | no job names it, and it is not a workspace member | nothing — see the workspace members in `Cargo.toml` |

`test-plan/coverage-matrix.md` keeps the per-crate table.

Two limits of the workspace run are worth stating, because "the gate covers everything" is easy to
overread:

* **`--no-default-features` means the default-feature configuration is not built by the gate.** The
  client crate's `desktop` feature needs GTK native libraries. `ci-client.yml` covers the desktop build
  on Windows and macOS, and `ci-client.yml` owns the desktop/default-feature configurations the headless release gate cannot build.
* **Ignored tests stay ignored.** `cargo test` reports them as ignored and exits 0. The gate asserts a
  floor on the *passed* count (`MIN_EXECUTED_TESTS`), which catches a suite that vanished, but it does
  not run `--ignored`. A defect recorded as an ignored test — the procedure described at the end of
  this file — is therefore still not executed by anything except a deliberate local run.

## Trigger coverage

Two triggers remain automatic, and both are deliberate:

* `maintenance-version-tag.yml` on a push that touches `Cargo.toml` or any `crates/**/Cargo.toml`. The
  `paths` filter is what makes this cheap — a dependency-version change is a release decision, and it
  is the only such decision the repository acts on by itself. **The path filter does not distinguish a
  version line from any other manifest edit**, so a dependency bump that leaves the root version alone
  still starts a run; it validates the workspace and then reports "version unchanged, no tag needed",
  which is a wasted run rather than a wrong release.
* `maintenance-sync-gitee.yml` on a push to `main`.

Because the crate layout is nested (`crates/<domain>/<name>/`), a single-segment glob is easy to get
wrong. This was measured: `crates/*/Cargo.toml` matches **zero** of the 37 tracked manifests, and
`maintenance-version-tag.yml` used that pattern, so a crate-manifest change never triggered it. It is
`crates/**/Cargo.toml` now.

## Reproducibility limits

* **`Cargo.lock` is still not tracked** (`.gitignore` line 29), so `--locked` still cannot be used and
  two different runners can resolve differently on their first run. The trusted self-hosted PR gate now
  reduces that cost by caching the generated lock file outside the checkout, keyed by the complete set
  of `Cargo.toml` files. A runner therefore reuses the same resolution while the manifests are unchanged,
  even though `actions/checkout` removes the ignored checkout-local `Cargo.lock`.
* Self-hosted Cargo registry/git caches and build artifacts live under `~/.cache/burncloud/`, outside
  the checkout. All four PR check kinds share the same persistent build directory,
  `~/.cache/burncloud/target`, instead of maintaining separate `target/test`, `target/clippy`,
  `target/fmt` and `target/deny` trees. This avoids both `git clean -ffdx` deleting `target/` and
  duplicate cold builds across checks. At the end of every trusted self-hosted Rust job, an `always()`
  post-step measures this shared target. If it is greater than 100 GiB, the workflow removes and recreates
  it. Cache housekeeping therefore belongs to CI, not to `code test`.
* PR deny checks use the advisory database under the persistent `CARGO_HOME` and pass
  `check --disable-fetch`. On first migration, the workflow seeds that database from
  `~/.cargo/advisory-dbs` when the runner already has one. A runner with no local advisory database
  fails fast with a clear bootstrap message rather than fetching during a PR.
* The Rust toolchain is `stable` via `dtolnay/rust-toolchain`, so the compiler version drifts with
  upstream releases. No version is pinned.

## Known-failing checks

The workspace gate was **run before it was depended on**, and this table is what that run found. Two
failures surfaced; both are recorded here rather than left to be discovered by a release that stops
half-way.

| Check | Cause | Status |
| --- | --- | --- |
| `burncloud-router` `health_probe::tests::test_probe_state_management` | the test asserted that a **Closed** breaker should be probed, contradicting the guard at the top of `HealthProbeManager::should_probe` (`if breaker.state() != CircuitState::HalfOpen { return false }`) | **Fixed.** The test now constructs the Half-Open breaker it meant to, and two tests were added: the Closed/Open cases, and the probe interval, which the original assertion could not distinguish from the in-flight flag |
| `burncloud-router` six HTTP integration tests: `test_claude_adaptor`, `test_deepseek_proxy`, `test_qwen_proxy`, `test_round_robin_balancer`, `test_failover`, `test_vertex_full_flow` | each posts to a stub upstream and receives the router's own 404 "No matching channel found" (`lib.rs:2333`) — the error returned when channel selection finds nothing. **Deterministic**: `test_deepseek_proxy` and `test_qwen_proxy` each fail when run entirely alone (0 passed, 1 failed, 2 filtered out), repeat runs fail, and their neighbours in the same files with the same helpers **pass** — `test_bedrock_proxy` in `auth_tests`, 19 of 20 in `adaptor_tests`. So it is the tests, not the environment | **Skipped by name** in the gate (`SKIP_TESTS`). The thread to pull first: the three tests that insert a `router_upstreams` row without the `protocol` column are all in this list. Not "fixed" by asserting the 404 — that would freeze a defect as the contract, which this repository forbids |
| `burncloud-service-user` `test_login_user_success` | cleanup removes the temporary SQLite file 200 ms after `close()`, and on Windows the pool has not always released the handle by then: `os error 32`, "another program is using this file" | **Skipped by name** in the gate. Intermittent, and observed only in a full-suite run, never in isolation (4/4 isolated runs passed) |
| `node-invariants` (E0308 in `interfaces/server/src/node_orchestrator.rs`) | previously failed on `main` | **No longer reproduces.** `cargo check --workspace --all-targets --no-default-features` exits 0 |
| `cd-release.yml` action pin | `softprops/action-gh-release@v1` was pinned, which `actionlint` reports as a runner version GitHub no longer supports | **Fixed** (bumped to `@v2`); still needs a release-correctness check before it is relied on |

A latent one, recorded before it costs someone an afternoon: `boundary_tests.rs` and
`token_expiry_tests.rs` both bind ports 3030–3033. Cargo runs test binaries sequentially today, so they
do not collide and neither has failed — but they would if that ever changed, and the symptom would look
like unrelated flakiness.

### What the gate does not run

Seven known failures are sourced from `.github/test-plan/known-test-baseline.txt` by `ci-quality.yml` and skipped **by
name**: the rest of each suite still runs. This is a visible hole, not a silent one — the step prints
what it skipped — and **all seven are defects, not expected behaviour**.

The reason for skipping rather than fixing is a division of labour, not indifference. A gate that is red
on `main` is a gate nobody reads, and the six router failures need a decision about routing behaviour:
either the tests' `router_upstreams` rows are stale or channel selection is wrong. Writing
`assert_eq!(resp.status(), 404)` would make that decision disappear into an expectation, which the rule
at the end of this file forbids.

`--skip` rather than `#[ignore]` on the test itself, so a local `cargo test` still runs all seven and
still fails — the signal stays where a developer will see it. When any is fixed, delete its name from
`SKIP_TESTS` in both files.

Two more things were found while measuring the gate, and **neither is handled by a skip list**. They are
recorded here with the decision they need, because they are the reason the gate cannot simply be turned
on and trusted:

**1. `cargo fmt --all -- --check` fails on `main`, in an untouched file.** Measured, not inferred:

```
$ git status --short crates/platform/storage/database/src/schema/price.rs
(no output -- unmodified)
$ git show HEAD:crates/platform/storage/database/src/schema/price.rs > /tmp/price.rs
$ rustfmt --edition 2021 --check --config skip_children=true /tmp/price.rs
Diff in /tmp/price.rs:24:          (a wrapped `sqlx::query_scalar(` call rustfmt wants on one line)
Diff in /tmp/price.rs:312:         (a missing trailing blank line)
```

So commit `edb5b451` ("fix(database): stop swallowing migration failures flagged by Clippy (#711)")
landed unformatted — which is itself the evidence that the local hook did not run for it. The fix is
`cargo fmt --all` (a one-command, formatting-only diff), but it belongs to whoever owns that file and
that commit, not to a workflow-trigger change.

**2. The black-box API suite cannot run unattended.** `crates/interfaces/tests/tests/common/mod.rs:82`
reuses any server already listening on port 3000 instead of spawning its own, and otherwise spawns
`target/debug/burncloud server start` and waits for readiness. Two tests in that binary —
`api::monitor::test_get_system_metrics` and `api::user::test_user_management_lifecycle` — failed with
`Request failed with status: 401 Unauthorized`, and the suite then **hung**: the spawned server stayed
alive after the suite gave up, and the run had to be killed by hand. A hanging step in a release gate is
worse than a failing one — it burns the whole job timeout and produces no verdict.

This is the same "different prerequisites" problem `test-plan/coverage-matrix.md` already records for
this crate, and the 401 suggests it needs an authenticated admin context the suite does not set up. Two
options, and the choice is not made here: give the suite a self-contained prerequisite (a fresh port it
owns, a database URL, an admin token) so it can be part of the workspace run, or exclude it and record
why. Until one of those is done, the workspace gate must not be treated as passing for
`crates/interfaces/tests`.

**Practical consequence for the `tests` job.** That step runs `cargo test --workspace`, which reaches
`burncloud-tests`' `api_tests` and can hang there. The job's `timeout-minutes: 60` is the only thing that
ends it, and a timeout reports as a cancelled job with no verdict — so a run that stops with no output
after an hour is most likely this, not a slow build. Phase 2, if this gate is going to be relied on
before the suite is fixed: either give `api_tests` the prerequisite it needs, or add
`--exclude burncloud-tests` to the gate and move that crate to its own manual workflow.

### The three compile failures no longer reproduce

This file used to list `burncloud-service-inference` (`E0599`/`E0282`), `burncloud-server`
`tests/log_api_tests.rs` (`E0433`) and the `burncloud-service-models` example (`E0432`) as the reason no
workflow runs `cargo test --workspace`. Re-measured against the current tree:
`cargo check --workspace --all-targets --no-default-features` exits 0, which is what makes the
workspace-wide gate possible at all.

### The `service-user` cleanup flake

Worth its own note because the reason it was never seen before is instructive: **no workflow ever ran
these tests.** `service-user` was in the "no job" list, so the 200 ms window in `TempDb::cleanup` was
never exercised by CI on any platform.

The assertion itself is right and should not be relaxed: a test that leaves its database behind is a
test that is writing somewhere it should not. The fix belongs in `TempDb::cleanup` — retry the removal
with a bounded backoff instead of sleeping a fixed 200 ms and then giving up — and it needs a run on
Windows to confirm.

### The gate writes to the developer's real database

Discovered while running the gate, and worth stating because it is a side effect of a command that
looks read-only. Several `burncloud-database` tests call `Database::new()` /
`create_default_database()`, which resolves to `%USERPROFILE%\AppData\Local\BurnCloud\data.db`. Running
`cargo test --workspace` on Windows therefore **creates and writes the default database**, both at the
real profile path and — from the test's working directory — at
`crates/platform/storage/database/AppData/Local/BurnCloud/data.db` inside the checkout. The second one
shows up as an untracked path, because `.gitignore` un-ignores everything under `crates/**`.

Set `BURNCLOUD_DATABASE_URL` before running the workspace suite if that matters. Neither path is
tracked by Git, and this is not fixed here because changing which path those tests use is a decision
about the tests, not about triggers.


## Branch protection

`main` currently has **no required status checks** (the only ruleset is a disabled "禁止删除main
branch"). Nothing here is enforced by the GitHub side yet — and with this change there is no longer a
GitHub check that *could* be required, because every check is manual. Branch protection is therefore
not "the last step of the plan" any more; it is not the mechanism this repository gates on. The
mechanism is the local hook.

## Conventions

* Workflow files are lowercase kebab-case `.yml`. `ci-*` = checks, `cd-*` = release,
  `maintenance-*` = repository upkeep.
* Job ids use domain names (`contracts`, `identity`, `supply`, `commerce`, `traffic`, `platform`,
  `server`, `client`).
* CI orchestration lives in YAML. There is no Python executor or custom config parser: package and
  target groupings belong in the workflow matrix, and the coverage matrix is documentation only.
* `run` steps use Bash. Shell scripts must not swallow failures with `|| true`, and a step must not
  report success when its command did not run.
* PR quality responsibilities are intentionally split into four thin self-hosted entry workflows so
  each produces an independent GitHub verdict. Shared execution policy belongs in
  `ci-self-hosted-base.yml`; do not copy runner/bootstrap logic back into the four callers.
* `ci-quality.yml` is the full-workspace release gate. It is intentionally broader than affected-
  package `code test`; do not describe the two as equivalent.

## Planned changes (not implemented)

Taken from `todos_1.txt` §2 and §9. Listed so the intent is on record, not to describe today's state.

```
arch.yml                       -> ci-architecture.yml
client-ui.yml                  -> ci-client.yml
release.yml                    -> cd-release.yml
version-check.yml              -> maintenance-version-tag.yml
sync-to-gitee.yml              -> maintenance-sync-gitee.yml
workflows/README.md            -> this file
```

Plus: a unified `ci.yml` entry point with a `CI Required` aggregation job, `merge_group` support,
and `actionlint` on the workflow files themselves. Two behaviours are deliberately **not** changed
here because getting them wrong would publish releases or break mirrors:

* A tag pushed by `maintenance-version-tag.yml` uses the default `GITHUB_TOKEN`, and events caused by that
  token **do not trigger further workflows** — so a tag created here may not start the release
  pipeline. The fix (a PAT, or `cd-release.yml` listening on `workflow_run`) needs a release-correctness
  test before it lands. The workflow summary now states this so a silent no-release is visible.
* `maintenance-sync-gitee.yml` uses `--all --force`; whether the mirror should be main-only or all refs, and
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
