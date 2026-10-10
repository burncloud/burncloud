# #828 Phase F1 — Identity Token / Traffic boundaries

Parent: #734. Prerequisites: #816, #822, #825.

## Ownership matrix

| Capability / data | Sole owner | Current implementation | Allowed public consumer |
| --- | --- | --- | --- |
| Token lifecycle, credential hashes, whitelist, rotations, validation | Identity | `identity/token` | Traffic Router, Server |
| Credential `quota_limit` and `used_quota`, atomic settlement | Identity | `identity/token` | Traffic via Identity public API |
| `order_type` and `price_cap_nanodollars` | Traffic | `traffic/database-router` | Traffic routing |
| Router logs, usage projections, video tasks and upstream/group persistence | Traffic | `traffic/database-router` / `traffic/service-router-log` | Traffic consumers |
| Per-request cost and pricing | Commerce | `commerce/billing` | Consumers via published billing contracts |

## Cargo dependency edges, before and after

Before (#825 baseline):

```text
interfaces/server -> identity/service-token
interfaces/service -> identity/service-token
traffic/database-router -> identity/service-token
identity/service-token -> platform/storage/database
traffic/service-router-log -> traffic/database-router
```

After (#829):

```text
interfaces/server -> identity/token
interfaces/service -> identity/token
traffic/database-router -> identity/token
identity/token -> platform/storage/database
traffic/service-router-log -> traffic/database-router
```

No backwards Identity -> Traffic crate dependency was introduced.

The dependency reversal implemented in #822 is present: Identity does not depend on Traffic's database crate. `traffic/database-router` still provides historical compatibility re-exports for credential types; their implementation belongs to Identity.

## Implemented crate move / package rename

```text
crates/identity/service-token  -> crates/identity/token
burncloud-service-token       -> burncloud-identity-token
burncloud_service_token       -> burncloud_identity_token
```

No changes to the `TokenService`, `RouterToken`, `RouterTokenModel`, `RouterTokenValidationResult`, `TokenRotationResult` API shapes, SQL, authorization behavior, quotas, or HTTP contracts.

References migrated in PR #829 (verify from PR diff):
- `Cargo.toml` workspace membership and dependency declaration
- `crates/traffic/database-router/{Cargo.toml,src/lib.rs,README.md}`
- `crates/interfaces/server/{Cargo.toml,src/api/token.rs,tests/security_invariants.rs}`
- `crates/interfaces/service/{Cargo.toml,src/lib.rs}`
- `crates/identity/service-token/{Cargo.toml,README.md,src/lib.rs,tests/token_credentials.rs,tests/token_service_entries.rs}`
- `maintenance-version-tag.yml`: retain the release-only 20-test Identity Token discovery floor after retiring `ci-quality.yml`; preserve the full-workspace release Fmt/Test/Clippy/Deny gate
- the retired-reference scan completed once on PR #829 and was intentionally removed from the reusable PR CI gate; no permanent repository-specific audit step is installed
- no tracked root `Cargo.lock` was found; verify dependency rules against current repo and CI

Note: a GitHub code-search index can lag the main branch; therefore index results alone are not adequate proof that every reference was migrated.

## Domain crate ten questions

1. **Owner:** Identity.
2. **Responsibility:** Credential lifecycle, validation, IP/security and quota state; not routing projections or request pricing.
3. **Data truth:** Identity-owned credential state, including `quota_limit` and `used_quota`.
4. **Public contract:** Existing `TokenService` and exported token record/result types, unchanged.
5. **Dependencies allowed:** Platform database and general utility crates.
6. **Consumers allowed:** Interfaces/Server and Traffic via stable Identity public API.
7. **Internal layout:** Existing `lib.rs` with private `repository` and its existing integration tests.
8. **Error/log/security:** Preserve the current `DatabaseError` mapping, key hashing, rotation and validation semantics.
9. **Evidence:** Existing credential tests, security invariants, focused consumers and full required PR checks.
10. **Examples:** Traffic may call `TokenService::validate`; Identity must not import Traffic router database implementation.

## Safety and CI acceptance

- Preserve all production schemas/migrations, token storage keys, public JSON/HTTP, error/authorization semantics.
- Do not move `traffic/database-router` or `traffic/service-router-log` as part of the Identity rename.
- Preserve `cargo test -p burncloud-identity-token --no-default-features` credential tests and the existing 20-test discovery floor.
- Validate `cargo fmt --all -- --check`, focused tests for affected packages, strict Clippy, `cargo deny check`, and `cargo run -- code test`.
- Check all actual required PR CI statuses on the current HEAD. A documentation-only pass is not evidence the implementation is ready.

## Acceptance evidence / outstanding verification

- [x] Identity / Traffic / Commerce Owner ownership matrix is documented above.
- [x] Crate rename, Cargo consumer updates and the original two credential integration test files are committed in PR #829.
- [x] Traffic router and router log implementation files were not modified in the PR.
- [x] Credential test discovery floor remains **20**; package target was renamed, floor not relaxed.
- [x] One-time tracked-tree retired-reference audit passed in [Fmt run 38015040312](https://github.com/burncloud/burncloud/actions/runs/38015040312): `git grep` found no old package/module/path references outside the migration plan and manifest regression fixture. That migration-specific step was subsequently removed from shared PR CI, as requested; the verified result remains historical evidence.
- [x] Test job [37972526930](https://github.com/burncloud/burncloud/actions/runs/37972526930) executed the full workspace, including `burncloud-identity-token` credential suites: `token_credentials.rs` **18 passed**, `token_service_entries.rs` **4 passed**, 0 failed/ignored/filtered in either suite. It also covered Server, Service, Traffic and other workspace consumers.
- [x] The actual `cargo run -p burncloud-code -- test --base origin/main` run detected the deleted former crate path as unclassified and deliberately selected the **entire workspace** rather than skipping deleted files. The manifest-scoped rename regression test passed, with no hidden skip arguments.
- [ ] Recheck Fmt, Code Test, Clippy, Deny on the **final latest HEAD** after this evidence update. The earlier `4cd7793` head passed all four, and the new tracked-tree audit's Fmt step passed on `d5b567a`; final PR HEAD must be checked again.
- [x] Source-level writer audit identified an existing cross-domain write in `traffic/database-router/src/log.rs` (`RouterLogModel::insert` updates `router_tokens.used_quota` by raw prompt+completion token count). This conflicts with Identity's ownership; tracked as **#830**. No unsafe behavior change is part of #829.
- [x] Separated the existing Data Truth violation into #830 per #734's Stop Conditions. `RouterLogService::insert` uses the side-effect-free `RouterDatabase::insert_log`; the legacy public `RouterLogModel::insert` has the cross-domain update. **Single-writer compliance is not yet proven and must be fixed/verified under #830**, not claimed by #829.
- [x] PR diff contains no schema/migration or Traffic router-log implementation changes. Token service method signatures, tests and database SQL were moved unchanged; the rename only updates Rust package/module references, not JSON/HTTP/error behavior.

## CI workflow retirements

PR #829 also deletes `ci-quality.yml`, `ci-integration.yml`, and `ci-client.yml` by request. The release `maintenance-version-tag.yml` has been updated to retain its full-workspace Fmt/Test/Clippy/Deny checks and existing 300/20/12 test-discovery floors; its former call to `ci-quality.yml` was removed. Dedicated manual real-PostgreSQL integration coverage and Windows/macOS client build workflows are **not** covered by those remaining PR gates and have been retired. The migration-specific `git grep` PR CI step was also reverted.

## Status

The rename-only migration and one-time repository-wide stale-reference audit are complete. All focused credential tests (22) and the entire workspace completed successfully on the earlier green commit. #830 tracks a **pre-existing** Protected Zone Data Truth issue requiring independent behavior investigation; do not conflate that fix with this structural PR. **Do not merge until every configured required CI check passes on the final HEAD.**
