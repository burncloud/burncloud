# #828 Phase F1 — Identity Token / Traffic boundaries

Parent: #734. Prerequisites: #816, #822, #825.

## Ownership matrix

| Capability / data | Sole owner | Current implementation | Allowed public consumer |
| --- | --- | --- | --- |
| Token lifecycle, credential hashes, whitelist, rotations, validation | Identity | `identity/service-token` | Traffic Router, Server |
| Credential `quota_limit` and `used_quota`, atomic settlement | Identity | `identity/service-token` | Traffic via Identity public API |
| `order_type` and `price_cap_nanodollars` | Traffic | `traffic/database-router` | Traffic routing |
| Router logs, usage projections, video tasks and upstream/group persistence | Traffic | `traffic/database-router` / `traffic/service-router-log` | Traffic consumers |
| Per-request cost and pricing | Commerce | `commerce/billing` | Consumers via published billing contracts |

## Observed Cargo dependency edges

```text
interfaces/server   -> identity/service-token
interfaces/service  -> identity/service-token
traffic/database-router -> identity/service-token
traffic/service-router-log -> traffic/database-router
identity/service-token -> platform/storage/database
```

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
- `.github/workflows/ci-quality.yml`: preserve the credential-test floor of 20; rename the package being tested, **never lower the floor**
- all other callers must be verified against the **branch tree**, not the default-branch code search index
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
- [ ] Verify the complete branch tree contains **zero** remaining references to the retired package name/path, including examples, scripts, docs and architecture/deny checks. GitHub code search may reflect the old default branch and is insufficient.
- [ ] Verify migrated tests are discovered and all behavior/SQL/auth/quota invariants remain intact; publish test counts and affected-consumer results.
- [ ] Verify `cargo run -- code test` plans the correct affected packages, including a rename-only move; no hidden skip or test-selection regression.
- [ ] Verify Cargo Fmt, Code Test, Clippy, Deny and all configured required checks pass on the **same latest HEAD**.
- [ ] Confirm source-level writer audit: Identity exclusively writes credential/quota state; Traffic exclusively owns `order_type` / `price_cap_nanodollars`; Commerce only calculates costs.
- [ ] Confirm no schema/migration, public HTTP/JSON/error semantics, or Protected Zone logic changed.

## Status

Implementation is present but final evidence is incomplete. **PR #829 must remain draft and issue #828 open until every outstanding verification above has positive evidence.**
