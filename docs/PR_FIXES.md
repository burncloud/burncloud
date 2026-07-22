# Local Fix Record

This note records the fixes made while validating the Windows local build.

## Changes

1. SQLite fresh-install migration
   - File: `crates/database/src/migration/sqlite_0017.rs`
   - Rebuild BOOLEAN columns as INTEGER based on the canonical table's actual
     column type, including upgrades where legacy tables were already removed.
   - Use a transaction for the rebuild and recover data left in a
     `*_boolfix` table by an interrupted earlier migration.

2. Inference integration test API drift
   - File: `crates/service/crates/inference/tests/integration_test.rs`
   - Replace the removed `RouterDatabase::get_upstream` calls with the current
     `ChannelProviderModel::list` API and assert the fields actually written by
     `InferenceService`.

3. Prometheus metric names
   - File: `crates/router/src/metrics.rs`
   - Remove the duplicate `burncloud` namespace so exported names remain
     `burncloud_requests_total`, `burncloud_tokens_*`, and similar.

4. Health probe test state
   - File: `crates/router/src/health_probe.rs`
   - Test the documented Closed -> Half-Open probe rules and concurrent probe guard.

5. Duplicate test attribute
   - File: `crates/router/src/smart_circuit_breaker.rs`
   - Remove the duplicate `#[test]` attribute from `test_multi_level_breaker`.

6. Windows SQLite test URLs
   - File: `crates/database/tests/migration_rename_tests.rs`
   - Build normalized SQLite URLs with the production-compatible
     `sqlite:///C:/...` absolute-path form.

7. Frontend delivery and Playground triggers
   - Files: `crates/server/src/lib.rs`, `crates/client/src/components/`, and
     `crates/client/crates/client-playground/src/lib.rs`.
   - Enable response compression, avoid duplicating the LiveView stylesheet in
     web layouts, and ensure Playground sends only after an explicit trigger.

8. Configurable dashboard port and HTTPS WebSocket
   - Files: `crates/client/crates/client-shared/src/api_client.rs` and
     `crates/client/src/lib.rs`.
   - Resolve the in-process API client port from `PORT`, including Playground
     chat requests, rather than hard-coding 3000. Generate `wss://` LiveView
     URLs when a reverse proxy reports `X-Forwarded-Proto: https`, and validate
     the host header before interpolation.

9. Debian operational support
   - Files: `deploy/burncloud.service`, `docs/debian-deployment.md`,
     `.github/workflows/release.yml`, and `src/main.rs`.
   - Add a hardened systemd unit, Debian source and Nginx deployment guide, and
     Linux x86_64 release artifacts. Persist an auto-generated `MASTER_KEY` in
     the service working directory instead of attempting to write beside the
     installed executable.

10. Internal control-plane protection and Docker build context
    - Files: `crates/router/src/lib.rs`, `deploy/docker-compose.yml`,
      `deploy/Dockerfile`, and deployment documentation.
    - Require `BURNCLOUD_INTERNAL_SECRET` for price-sync, circuit-breaker,
      metrics, and internal health routes. Build Docker from the repository
      root so a clean checkout contains all workspace sources.

11. CLI authentication contract and credential permissions
    - File: `src/cli/client.rs`.
    - Send the `username` field expected by `/api/auth/login`, honor
      `BURNCLOUD_SERVER_URL` or `PORT`, and restrict saved credentials to mode
    `0600` on Unix.

12. Windows-safe database integration tests and exchange-rate coverage
    - Files: `crates/database/crates/*/tests/` and
      `crates/router/src/exchange_rate.rs`.
    - Normalize temporary SQLite paths to absolute `sqlite:///C:/...` URLs,
      remove hard-coded `/tmp` state, expose the supported rate override API,
      and deserialize remote rates into a typed map.

13. Current-schema router integration tests and adaptor routing
    - Files: `crates/router/tests/` and `crates/router/src/lib.rs`.
    - Seed `channel_providers` and `channel_abilities` instead of removed
      upstream/group tables. Allow OpenAI-compatible requests to use provider
      adaptors, retain native Anthropic path restrictions, and preserve Vertex
      as a distinct protocol.

14. Provider response-quality correctness and failover diagnostics
    - Files: `crates/router/src/lib.rs` and
      `crates/router/src/response_quality.rs`.
    - Validate provider-native payloads before converting them to OpenAI
      format, accept Vertex `streamGenerateContent` array responses, and retain
      the response-quality reason in the final all-upstreams-failed error.

15. Authentication and internal-control integration coverage
    - Files: `crates/tests/tests/api/`, `crates/tests/tests/common/mod.rs`, and
      `crates/router/tests/common.rs`.
    - Obtain real JWTs through public register/login endpoints for protected
      API tests, and include `x-internal-secret` in test-only control-plane
      requests.

16. Client all-feature entry-point compatibility
    - Files: `crates/client/src/main.rs`, `app.rs`, and `lib.rs`.
    - Provide a dedicated web launcher and make the desktop entry point take
      precedence when desktop and web features are enabled together.

17. Poisoned empty-response counter recovery
    - File: `crates/router/src/lib.rs`.
    - Recover poisoned read/write guards with a warning rather than panicking
      all subsequent router requests.

18. Reproducible dependency resolution and Docker Rust version
    - Files: `Cargo.lock`, `.gitignore`, and `deploy/Dockerfile`.
    - Track the workspace lock file, build release artifacts with `--locked`,
      and use Rust 1.88 in Docker because the resolved `home` dependency
      requires Rust 1.88 (and Dioxus requires at least Rust 1.83).

19. Release tag and package version consistency
    - File: `.github/workflows/release.yml`.
    - Stop rewriting only manifests that happen to contain version `0.1.0`.
      Read Cargo metadata instead and reject Windows or Linux release jobs when
      the tag does not match the root `burncloud` package version.

20. CLI version and successful help/version exits
    - File: `src/cli/commands.rs`.
    - Report the root package version for `--version` and update checks, and
      treat Clap's help/version display results as successful exits.

21. Authenticated CLI price-cache refresh
    - File: `src/cli/price.rs`.
    - Add `x-internal-secret` to the protected price-cache refresh request when
      `BURNCLOUD_INTERNAL_SECRET` is configured, with regression coverage for
      configured and blank values.

22. Streaming token-counter clone cleanup
    - File: `crates/router/src/lib.rs`.
    - Remove a duplicate `Arc::clone` in the passthrough streaming hot path that
      was immediately shadowed and performed an unnecessary atomic operation.

## Verification

Run from a Visual Studio Developer PowerShell with the Rust toolchain on `PATH`:

```powershell
cargo check --workspace --lib
cargo check --workspace --all-targets
cargo test -p burncloud-database --tests
cargo test -p burncloud-router --lib
cargo test --offline --workspace --all-targets --all-features --no-fail-fast
cargo clippy --offline --workspace --all-targets --all-features
```

Results on Windows/MSVC:

- `cargo check --workspace --lib`: passed.
- `cargo check --workspace --all-targets`: passed.
- `cargo test -p burncloud-database --tests`: passed (88 tests).
- `cargo test -p burncloud-router --lib`: passed (198 tests).
- Full workspace/all-target/all-feature test command: passed.
- API integration target on an isolated SQLite database: passed (73 run,
  105 explicitly ignored, 0 failed).
- Full workspace/all-target/all-feature Clippy: passed with baseline warnings.
- Release build: passed with `cargo build --release`.
- Runtime probe: `http://127.0.0.1:3000/health` returned `200 ok`.
- Frontend delivery probe: Brotli reduces the LiveView HTML shell from
  `164,093` bytes to `32,423` bytes on the wire.
- Locked root binary build and tests: passed (7 tests).
- Locked auto-update tests: passed (3 tests).
- Locked router library tests: passed (198 tests).
- Focused locked root-binary Clippy: passed with baseline warnings.
- CLI probes: `burncloud --version` reports `0.1.17`; `--version` and `--help`
  both exit with status 0.
- Release metadata gate: root package version resolved as `0.1.17` from the
  tracked lock file.

The build still emits pre-existing unused-import, unused-variable, and dead-code
warnings in the router and E2E test crates; they do not block compilation.

## Known baseline formatting issue

`cargo fmt --all -- --check` currently reports pre-existing formatting and
trailing-whitespace issues in unrelated files. The fix changes are kept scoped
to the files listed above.
