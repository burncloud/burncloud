# CI coverage matrix

Scope: the 32 packages in this workspace (`cargo metadata --no-deps`), plus the root package's CLI.
This matrix is **documentation**; the executable grouping lives in the workflow YAML, so there is no
second copy of the commands to keep in sync.

**This file was rewritten when the workspace gate landed, because its central claim stopped being
true.** It used to report per-crate jobs, and it counted **21 crates with no job**. There are no
per-crate jobs any more: `ci-quality.yml` runs `cargo test --workspace --no-default-features`, so a
crate is verified because it is a **workspace member**, not because a job names it. The 21 "no job"
rows were an artifact of a hand-maintained crate list, not a statement about those crates' tests.

It answers one question per crate: *if I change this, does the gate run its tests?*

| State | Meaning |
| --- | --- |
| **tested** | the gate compiles the package's tests and runs them, as a workspace member |
| *compile only* | the gate type-checks the package, but its tests are not executed (the desktop feature configurations the gate cannot build) |
| **not built** | the package is not a workspace member, so nothing compiles it |

## Current state

`cargo check --workspace --all-targets --no-default-features` exits 0 on `main`, and the workspace run
executes the workspace test executables. The three compile failures this file used to record under
"Deliberately not in CI" no longer reproduce; they are listed at the bottom with that correction.

Every row below is a workspace member unless the row says otherwise, so every row is **tested**.

| Crate | Package | Tests present | Verified by |
| --- | --- | --- | --- |
| `crates/commerce/contracts` | `burncloud-commerce-contracts` | 2 unit + 3 integration | **tested** — workspace run |
| `crates/supply/contracts` | `burncloud-supply-contracts` | 2 integration | **tested** — workspace run |
| `crates/traffic/contracts` | `burncloud-traffic-contracts` | 2 integration | **tested** — workspace run |
| `crates/interfaces/common` | `burncloud-common` | 3 integration | **tested** — workspace run |
| `crates/commerce/database-billing` | `burncloud-database-billing` | 1 unit + 1 integration | **tested** — workspace run |
| `crates/commerce/service-billing` | `burncloud-service-billing` | 8 unit | **tested** — workspace run |
| `crates/supply/channel` | `burncloud-supply-channel` | 2 unit + 5 integration | **tested** — workspace run |
| `crates/identity/database-user` | `burncloud-database-user` | 1 integration | **tested** — workspace run |
| `crates/traffic/router` | `burncloud-router` | 28 unit + 20 integration | **tested** — workspace run |
| `crates/interfaces/server` | `burncloud-server` | 6 unit + 7 integration | **tested** — workspace run |
| `crates/platform/node` | `burncloud-node-runtime` | 6 unit + 13 integration | **tested** — workspace run |
| root (`crates/interfaces/cli`) | `burncloud` | none | *compile only* — workspace run compiles it; `ci-client.yml` builds it |
| `crates/interfaces/client` | `burncloud-client` | 13 unit | *compile only* — `ci-client.yml` (Windows/macOS desktop); the gate excludes the `desktop` feature |
| `crates/trust/inference` | `burncloud-service-inference` | 1 integration | **tested** — workspace run |
| `crates/identity/service-user` | `burncloud-service-user` | 1 unit | **tested** — workspace run, with a discovery floor |
| `crates/identity/service-token` | `burncloud-service-token` | none | **tested** — workspace run, with a discovery floor |
| `crates/supply/model` | `burncloud-supply-model` | model capability + resolver coverage | **tested** — workspace run |
| `crates/traffic/database-router` | `burncloud-database-router` | 2 integration | **tested** — workspace run |
| `crates/traffic/service-router-log` | `burncloud-service-router-log` | none | **tested** — workspace run |
| `crates/traffic/router-aws` | `burncloud-router-aws` | 1 unit | **tested** — workspace run |
| `crates/platform/storage/database` | `burncloud-database` | 7 integration | **tested** — workspace run |
| `crates/platform/storage/database-sys` | `burncloud-database-sys` | 1 unit + 1 integration | **tested** — workspace run |
| `crates/platform/storage/download` | `burncloud-download` | none | **tested** — workspace run |
| `crates/platform/storage/download-aria2` | `burncloud-download-aria2` | 1 unit | **tested** — workspace run |
| `crates/platform/configuration/setting` | `burncloud-service-setting` | none | **tested** — workspace run |
| `crates/platform/lifecycle/installer` | `burncloud-installer` | 6 unit | **tested** — workspace run |
| `crates/platform/lifecycle/loops` | `burncloud-loops` | none | **tested** — workspace run |
| `crates/platform/lifecycle/update` | `burncloud-auto-update` | 1 unit | **tested** — workspace run |
| `crates/platform/observability/ip` | `burncloud-service-ip` | none | **tested** — workspace run |
| `crates/platform/observability/monitor` | `burncloud-service-monitor` | none | **tested** — workspace run |
| `crates/interfaces/service` | `burncloud-service` | none | **tested** — workspace run |
| `crates/interfaces/tests` | `burncloud-tests` | 10 integration | **tested** — workspace run |

Totals: **31 tested**, **2 compile only**, **0 with no job**.

The test column is a static count of files containing `#[test]` and of `tests/*.rs`. It says nothing
about whether those assertions are meaningful — the planning document's warning about
"test count is not coverage" applies here. Two rows deserve the qualification stated rather than
implied: `burncloud-service-token` and `burncloud-service-user` carry explicit discovery floors in
`ci-quality.yml`, because `cargo test` exits 0 for a suite that ran nothing, and a renamed test
directory would otherwise look like a pass.

## Gaps by priority

The planning document (`todos_1.txt` §5) assigns a priority and a type to each crate. That list is
input for test authoring; this matrix only tracks **execution**.

### The gap that is left, and it is no longer "no job"

Execution is no longer the gap. **Presence of tests is.** These P0 crates are now compiled and their
suites run, and the suites are empty or near-empty:

### P0 crates whose tests now run but barely exist

| Crate | Why it matters |
| --- | --- |
| `crates/traffic/database-router` | credential validation, settlement, balance writes |
| `crates/identity/service-user` | registration, login, JWT, first-user admin policy |
| `crates/identity/service-token` | token lifecycle |
| `crates/supply/model` | model capability persistence + resolver/HF implementation |

### P0 crates with no tests at all

`burncloud-service-token` (none in-crate; its tests live in `tests/`), `burncloud-service-router-log`,
`burncloud-service-setting`.

`crates/supply/model` now owns the real `model_capabilities` persistence from #621; the former stubbed `ModelDatabase` no longer exists. Historical note:
`Ok(())` / `Ok(None)` / `Ok(vec![])`, so it is a placeholder, not an implementation. Writing tests
against it now would either fail or freeze the stub as expected behaviour. It is tracked in #621.

## Deliberately not in CI

A permanently red check is worse than no check, because it trains reviewers to ignore failures. That
principle has not changed; the list of things it applies to has shrunk to almost nothing.

**Correction:** this section used to exclude three targets for compile failures. Re-measured against
the current tree, `cargo check --workspace --all-targets --no-default-features` exits 0, so all three
compile and their tests run in the gate:

* `burncloud-service-inference` `tests/integration_test.rs` — **compiles now** (was E0599 + E0282)
* `burncloud-server` `tests/log_api_tests.rs` — **compiles now** (was E0433)
* `burncloud-supply-model` examples — **compile as part of the merged Supply Model crate**

What remains excluded, and why:

* **Two tests skipped by name** in the gate's `SKIP_TESTS`, both recorded in `.github/README.md` as
  defects rather than expected behaviour: `test_claude_adaptor` (deterministically receives the
  router's 404 "No matching channel found" where it expects 200) and `test_login_user_success` (a
  Windows file-handle race in its own cleanup). Their **suites still run** — only the two tests are
  skipped — so this is a narrower hole than the crate-level exclusions it replaces.
* `crates/interfaces/tests` — mixes offline API, browser, installer, real-provider and cloud-resource
  targets with different prerequisites. Its offline targets run in the gate; the ones that need a
  browser, a provider account or cloud credentials are run when someone runs them, which is the same
  position as before.
* `crates/platform/storage/download-aria2` — depends on an aria2 daemon. It runs in the gate as a
  workspace member; the test skips when the daemon is absent, so its 1 unit test may report as
  filtered rather than passed.

## How to add tests to the gate

1. Confirm the tests pass **locally on `main`** with the exact command you intend to add. A job that
   is born red is not coverage.
2. If the package is a workspace member, its tests already run. Nothing to add to a workflow.
3. If it is **not** a workspace member, add it to `members` in the root `Cargo.toml`. That single act
   is what puts it in the gate; there is no per-crate job to edit and no `paths` filter to update.
4. Update this matrix in the same change.
5. For a suite whose disappearance would be silent — anything asserting a security or billing
   invariant — add a discovery floor next to the ones in `ci-quality.yml` rather than trusting that
   `--workspace` going green means the suite ran.

Note the failure mode this repository already hit once and which is now structurally impossible: a
`paths` filter or a crate list that matches nothing, producing a job that never runs and a green tick
that proves nothing. `--workspace` reads the manifest, so the two cannot disagree.
