# burncloud-identity-user

Identity-owned vertical User crate.

This crate consolidates the former `burncloud-database-user` and
`burncloud-service-user` technical layers under one business Owner.

It owns the existing account/role/API-key/recharge/password-reset persistence
and the existing registration, login, bcrypt, JWT and traffic-class behavior.

This migration does not change schema, migrations, credential projection,
balance semantics, authentication semantics, or HTTP contracts.

The Cargo package is `burncloud-identity-user`. During #793 the Rust library
target intentionally remains `burncloud_service_user` so structural migration
is isolated from the repository-wide import rename.

## Domain Crate 10 问

1. **Owner** — Identity. Business capability: user account, authentication, credentials, roles and
   wallet settlement. The Cargo package is `burncloud-identity-user`; the Rust library target remains
   `burncloud_service_user` (`Cargo.toml` `[lib] name`), so consumers import
   `burncloud_service_user::…`. This crate consolidates the former `burncloud-database-user` and
   `burncloud-service-user` technical layers under one Owner.

2. **Responsibility** — registration, login and bcrypt hashing/verification; JWT issuance and
   validation; the first-user-is-admin policy; role binding and lookup; account read/write; API-key
   CRUD; password-reset token persistence; recharge records and balance crediting; dual-currency
   wallet debits; and resolving a user's DiffServ traffic colour for the router. **Not responsible
   for**: Platform DDL, migrations and seed data; Traffic routing and scheduling (this crate only
   returns a `TrafficColor`, the router stays colour-agnostic); Commerce pricing and cost
   calculation; the `identity/token` credential spend quota (`quota_limit` / `used_quota`); HTTP/CLI
   presentation and the OAuth code exchange (only the authorize URL is built here).

3. **Data Truth** — `user_accounts` (including the wallet columns `balance_usd` / `balance_cny`),
   `user_roles`, `user_role_bindings`, `user_api_keys`, `user_recharges` and
   `password_reset_tokens`. Balances and recharge amounts are `i64` nanodollars. The wallet columns
   have a single writer module, `src/wallet.rs`. Table DDL is Platform-owned
   (`migrations/0010_rename_tables.sql`, `0012_password_reset_and_google_id.sql`);
   `UserDatabase::init` only issues `CREATE TABLE IF NOT EXISTS` for `user_roles`,
   `user_role_bindings` and `user_recharges`, runs best-effort `ALTER TABLE` column migrations,
   seeds default roles and assigns roles to orphan users. Historical migrations are not rewritten
   here.

4. **Public contract** — from the crate root: `UserService`, `JwtSecret`, `AuthToken`,
   `UserServiceError`; `BalanceModel` (`deduct_usd`, `deduct_cny`, `deduct_dual_currency`,
   `deduct_dual_currency_router_legacy`); and the persistence surface `UserDatabase` (spec-aligned
   alias `UserAccountModel`), `UserAccount`, `UserAccountInput`, `UserApiKey`, `UserApiKeyModel`,
   `UserApiKeyInput`, `UserApiKeyUpdateInput`, `UserRecharge`, `PasswordResetDatabase`,
   `PasswordResetToken`. New code should prefer `UserService` / `BalanceModel` over raw repository
   calls.

5. **Allowed dependencies** — `burncloud-traffic-contracts` (only for `TrafficColor`),
   `burncloud-database`, plus `sqlx`, `bcrypt`, `jsonwebtoken`, `chrono`, `uuid`, `rand`, `dashmap`,
   `serde`, `thiserror` and `tracing`. This crate must not depend on Commerce, Supply or interface
   implementation crates, and must not reach its own credential data through Traffic's database
   crate.

6. **Allowed consumers** — `crates/interfaces/server`, `crates/interfaces/cli`,
   `crates/interfaces/tests` and `crates/traffic/router`. New cross-domain dependencies require
   architecture review; the existing ones are migration debt, not precedent.

7. **Internal structure** — business modules at the crate root, SQL and row mapping in a private
   `repository` module:

   ```text
   src/
   ├── lib.rs                 UserService, JwtSecret, AuthToken, UserServiceError, traffic colour
   ├── wallet.rs              BalanceModel — single writer of the balance columns
   └── repository/
       ├── mod.rs             UserDatabase (alias UserAccountModel), init, roles, balances
       ├── common.rs          private helpers
       ├── user_account.rs    UserAccount, UserAccountInput
       ├── user_api_key.rs    UserApiKey, UserApiKeyModel, UserApiKeyInput, UserApiKeyUpdateInput
       ├── user_recharge.rs   UserRecharge
       └── password_reset.rs  PasswordResetDatabase, PasswordResetToken
   ```

   `docs/` keeps the legacy `burncloud-database-user` / `burncloud-service-user` READMEs.

8. **Errors / logs / security** — service calls return `UserServiceError` (`DatabaseError`,
   `UserAlreadyExists`, `UserNotFound`, `InvalidCredentials`, `HashError`, `TokenError`,
   `TokenValidationError`, `ConfigError`); persistence propagates
   `burncloud_database::DatabaseError` and leaves presentation to callers. Logging is `tracing` at
   info/warn/debug, and SQL values are bound rather than interpolated. `JwtSecret` keeps its inner
   value private, redacts `Debug` to `JwtSecret([REDACTED])` and rejects empty or whitespace-only
   secrets; `password_hash` is `#[serde(skip_serializing)]` and prints `[REDACTED]` in `Debug`;
   `UserApiKey` exposes its key only through `mask_key` (`sk-****<last 4>`, or `****` when
   malformed). Secrets are data, not log fields.

9. **Proof** — SQLite executes `src/lib.rs` `mod tests`, `tests/identity_round_trip.rs` (including
   the wallet owner suite), `tests/identity_rules.rs`, `tests/user_rules.rs`,
   `tests/credential_projection.rs` (redaction, no database) and the `tests/no_default_database.rs`
   source guard. PostgreSQL executes `tests/postgres_round_trip.rs`:
   `accounts_round_trip_through_the_real_postgres_schema`,
   `a_missing_account_is_none_on_postgres_not_an_error`,
   `identity_wallet_preserves_debit_variants_on_real_postgres` and
   `wallet_parallel_cross_currency_charges_never_double_spend_on_postgres`; on
   `GITHUB_ACTIONS=true` the `github_actions_runs_real_identity_wallet_contracts` test starts a
   disposable `postgres:16` container, and without `BURNCLOUD_TEST_POSTGRES_URL` the suite is skipped
   with a stated message rather than silently counted as a pass.

10. **Correct / incorrect calls**

    ```text
    Correct:   interfaces   -> UserService (auth) and UserDatabase / UserApiKeyModel (persistence)
    Correct:   Traffic      -> BalanceModel (settlement), UserService::resolve_traffic_class (colour)
    Correct:   JWT caller   -> JwtSecret (validated); the raw secret is never reconstructed
    Incorrect: another domain -> SELECT/UPDATE user_accounts, user_api_keys or user_recharges directly
    Incorrect: writing balance_usd / balance_cny outside src/wallet.rs
    Incorrect: serializing or logging password_hash, the raw UserApiKey.key, or the JWT secret
    ```
