# CI coverage matrix

Scope: the 34 packages in this workspace (`cargo metadata --no-deps`), plus the root package's CLI.
This matrix is **documentation**; the executable grouping lives in the workflow YAML, so there is no
second copy of the commands to keep in sync.

It answers one question per crate: *if I change this, does any CI job actually run its tests today?*
Three states appear below, and they are not the same thing:

| State | Meaning |
| --- | --- |
| **tested** | a CI job runs `cargo test` for this package |
| *compile only* | a CI job runs `cargo check`/`clippy`/desktop build, so type errors surface but no assertion runs |
| **no job** | no workflow names this package; nothing about it is verified by CI |

## Current state

The `CI job` column refers to the split introduced in `security-billing-invariants.yml` (five jobs:
`formatting`, `node-invariants`, `billing-invariants`, `security-invariants`, `migration-contracts`).
Values marked *(after #623)* take effect only once that PR is merged.

| Crate | Package | Tests present | CI job |
| --- | --- | --- | --- |
| `crates/commerce/contracts` | `burncloud-commerce-contracts` | 2 unit + 3 integration | **tested** — `migration-contracts` *(after #623)* |
| `crates/supply/contracts` | `burncloud-supply-contracts` | 2 integration | **tested** — `migration-contracts` *(after #623)* |
| `crates/traffic/contracts` | `burncloud-traffic-contracts` | 2 integration | **tested** — `migration-contracts` *(after #623)* |
| `crates/interfaces/common` | `burncloud-common` | 3 integration | **tested** — `migration-contracts` *(after #623)* |
| `crates/commerce/database-billing` | `burncloud-database-billing` | 1 unit + 1 integration | **tested** — `billing-invariants` *(after #623)* |
| `crates/commerce/service-billing` | `burncloud-service-billing` | 8 unit | **tested** — `billing-invariants` *(after #623)* |
| `crates/supply/database-channel` | `burncloud-database-channel` | 1 integration | **tested** — `migration-contracts` *(after #623)* |
| `crates/identity/database-user` | `burncloud-database-user` | 1 integration | **tested** — `migration-contracts` *(after #623)* |
| `crates/traffic/router` | `burncloud-router` | 28 unit + 20 integration | **tested** — `billing-invariants`, `node-invariants` |
| `crates/interfaces/server` | `burncloud-server` | 6 unit + 7 integration | **tested** — `security-invariants`, `node-invariants` |
| `crates/platform/node` | `burncloud-node-runtime` | 6 unit + 13 integration | **tested** — `node-invariants` |
| root (`crates/interfaces/cli`) | `burncloud` | none | *compile only* — `client-ui.yml` |
| `crates/interfaces/client` | `burncloud-client` | 13 unit | *compile only* — `client-ui.yml` (Windows/macOS desktop) |
| `crates/trust/inference` | `burncloud-service-inference` | 1 integration | **no job** |
| `crates/identity/service-user` | `burncloud-service-user` | 1 unit | **no job** |
| `crates/identity/service-token` | `burncloud-service-token` | none | **no job** |
| `crates/supply/database-model` | `burncloud-database-model` | none | **no job** |
| `crates/supply/service-channel` | `burncloud-service-channel` | none | **no job** |
| `crates/supply/service-models` | `burncloud-service-models` | 1 unit + 1 integration | **no job** |
| `crates/traffic/database-router` | `burncloud-database-router` | 2 integration | **no job** |
| `crates/traffic/service-router-log` | `burncloud-service-router-log` | none | **no job** |
| `crates/traffic/router-aws` | `burncloud-router-aws` | 1 unit | **no job** |
| `crates/platform/storage/database` | `burncloud-database` | 7 integration | **no job** |
| `crates/platform/storage/database-sys` | `burncloud-database-sys` | 1 unit + 1 integration | **no job** |
| `crates/platform/storage/download` | `burncloud-download` | none | **no job** |
| `crates/platform/storage/download-aria2` | `burncloud-download-aria2` | 1 unit | **no job** |
| `crates/platform/configuration/setting` | `burncloud-service-setting` | none | **no job** |
| `crates/platform/lifecycle/installer` | `burncloud-installer` | 6 unit | **no job** |
| `crates/platform/lifecycle/loops` | `burncloud-loops` | none | **no job** |
| `crates/platform/lifecycle/update` | `burncloud-auto-update` | 1 unit | **no job** |
| `crates/platform/observability/ip` | `burncloud-service-ip` | none | **no job** |
| `crates/platform/observability/monitor` | `burncloud-service-monitor` | none | **no job** |
| `crates/interfaces/service` | `burncloud-service` | none | **no job** |
| `crates/interfaces/tests` | `burncloud-tests` | 10 integration | **no job** |

Totals: **11 tested** (3 today, 8 more once #623 is merged), **2 compile only**, **21 with no job**.

The test column is a static count of files containing `#[test]` and of `tests/*.rs`. It says nothing
about whether those assertions are meaningful — the planning document's warning about
"test count is not coverage" applies here.

## Gaps by priority

The planning document (`todos_1.txt` §5) assigns a priority and a type to each crate. That list is
input for test authoring; this matrix only tracks **execution**.

### P0 crates with tests that no job runs

| Crate | Why it matters |
| --- | --- |
| `crates/traffic/database-router` | credential validation, settlement, balance writes |
| `crates/identity/service-user` | registration, login, JWT, first-user admin policy |
| `crates/identity/service-token` | token lifecycle |
| `crates/supply/database-model` | stubbed CRUD (see note below) |

### P0 crates with no tests at all

`burncloud-service-token`, `burncloud-service-router-log`, `burncloud-service-channel`,
`burncloud-service-setting`.

`crates/supply/database-model` deserves a separate line: its `ModelDatabase` methods return
`Ok(())` / `Ok(None)` / `Ok(vec![])`, so it is a placeholder, not an implementation. Writing tests
against it now would either fail or freeze the stub as expected behaviour. It is tracked in #621.

### Blockers before these can join CI

1. **`crates/identity/service-user`** — the planning document states its existing database test calls
   `create_default_database`, which touches a shared database rather than an isolated one. It needs a
   temporary database first (the pattern already used by `database-user`'s
   `tests/identity_round_trip.rs`).
2. **`crates/trust/inference`** — `tests/integration_test.rs` does not compile
   (`RouterDatabase::get_upstream` is missing, E0599 + E0282). Fix or retire the target before adding
   it to a required suite.
3. **`crates/interfaces/tests`** — mixes offline API, browser, installer, real-provider and
   cloud-resource targets with different prerequisites. It needs splitting by prerequisite before
   anything can be made required.
4. **`crates/platform/storage/download-aria2`** — depends on an aria2 daemon; belongs with the
   controlled-external class, not the default PR path.

## Deliberately not in CI

A permanently red check is worse than no check, because it trains reviewers to ignore failures.
The following are excluded until their own defects are fixed (see `.github/README.md`):

* `burncloud-service-inference` `tests/integration_test.rs` — E0599 + E0282
* `burncloud-server` `tests/log_api_tests.rs` — E0433 (`test_utils`)
* `burncloud-service-models` example — E0432 (`ModelInfo`)
* the `node-invariants` job stays red on `main` because of `node_orchestrator.rs:855,911` (E0308)

## How to add a crate to a suite

1. Confirm the tests pass **locally on `main`** with the exact command you intend to add. A job that
   is born red is not coverage.
2. Add the package to the appropriate job in `security-billing-invariants.yml` (or the relevant
   workflow) and add its path to the `paths` filter — a job that never triggers is the failure mode
   this repository has already hit once (`crates/*/Cargo.toml` matching zero manifests).
3. Update this matrix in the same PR.
4. After the PR runs, confirm the job executed. A workflow filtered out by `paths` reports nothing,
   which looks like success but proves nothing.
