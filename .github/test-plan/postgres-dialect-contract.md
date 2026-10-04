# PostgreSQL dialect contract

What the two backends are allowed to differ on, what each crate's Rust code branches on, and which of
those branches has been executed. Written because `traffic/database-router` and
`supply/database-channel` have SQLite coverage and no PostgreSQL coverage, and adding PostgreSQL tests
without knowing where the branches are would mean guessing which queries matter.

The audit below was done by reading the source and comparing the two migration sets, because this
machine has neither a PostgreSQL server nor a container runtime. Every claim names where it came from.

## 1. Where the Rust code branches on the backend

Measured by counting lines that read the backend kind or build SQL conditionally:

| Crate | File | Backend-dependent lines |
| --- | --- | --- |
| `traffic/database-router` | `log.rs` | 60 |
| | `token.rs` | 36 |
| | `lib.rs` | 13 |
| | `router_video_task.rs` | 2 |
| `supply/database-channel` | `channel_provider.rs`, `channel_ability.rs`, `channel_protocol_config.rs` | the bulk of the remainder |

163 in total. They fall into four kinds, and only the first two are mechanical:

1. **Placeholders.** `ph(is_postgres, n)` renders `$n` or `?`; `phs(is_postgres, count)` renders a whole
   list. `adapt_sql` rewrites `?` into `$n` for PostgreSQL.
2. **Quoted identifiers.** `` `group` `` on SQLite, `"group"` on PostgreSQL, because `group` is reserved in
   both. The same applies to `type`.
3. **Different expressions for the same result.** The clearest example, in `log.rs`:
   `EXTRACT(EPOCH FROM created_at)::BIGINT >= $n` versus
   `CAST(strftime('%s', created_at) AS BIGINT) >= ?`.
4. **Different statements for the same operation.** `channel_protocol_config.rs` writes `is_default = FALSE`
   on PostgreSQL and `is_default = 0` on SQLite; `channel_provider.rs` and `router_video_task.rs` retrieve
   the new id with `RETURNING id` on PostgreSQL and `SELECT last_insert_rowid()` on SQLite.

## 2. The timestamp convention is not uniform, and that is the real hazard

Kind 3 above only works if the column's type matches the expression. Comparing the two migration sets
column by column found **35 timestamp-shaped columns whose declared types differ**, and they fall into two
conventions:

| Convention | SQLite | PostgreSQL | Example tables |
| --- | --- | --- | --- |
| **Millisecond integer** | `INTEGER` | `BIGINT`, default `(EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT` | `billing_plans`, `billing_subscriptions`, `billing_prices`, `prices`, `channel_protocol_configs`, `tokens`, `user_api_keys`, `router_tokens` |
| **Timestamp** | `TEXT DEFAULT CURRENT_TIMESTAMP` | `TIMESTAMP DEFAULT CURRENT_TIMESTAMP` | `router_logs`, `user_recharges`, `sys_downloads`, `router_request_logs`, `router_video_tasks` |

So a query written against one convention is wrong against the other, and nothing in the Rust code states
which convention a given table follows.

**Checked, and it is correct here.** The `log.rs` branch was the one worth doubting, because
`EXTRACT(EPOCH FROM created_at)` fails on a `BIGINT` column. `router_logs.created_at` is `TIMESTAMP` on
PostgreSQL and `TEXT` on SQLite, so the branch is right. The doubt was recorded and then disproved by
reading the migration rather than by assuming either way.

**What this means for new code.** A query added against `router_logs` can use the timestamp convention; one
added against `prices` or `channel_protocol_configs` must use the millisecond convention. Getting it
backwards fails at runtime, not at compile time, and only on one backend.

## 3. What has actually been executed

| Branch | Executed? | Where |
| --- | --- | --- |
| SQLite placeholder path (`?`) | yes | every existing test in both crates |
| SQLite quoted identifiers (`` `group` ``) | yes | `sqlite_row_mapping.rs`, the new ability and protocol-config tests |
| SQLite `last_insert_rowid()` id retrieval | yes | `channel_provider.rs` through `create`, exercised by `create` in the tests |
| **PostgreSQL placeholder path (`$n`)** | **no** | nothing connects with a `postgres://` URL except the Identity test in `ci-integration.yml` |
| **PostgreSQL quoted identifiers (`"group"`)** | **no** | as above |
| **PostgreSQL `RETURNING id`** | **no** | as above |
| **PostgreSQL `is_default = FALSE`** | **no** | as above |
| **PostgreSQL epoch expressions on both conventions** | **no** | as above |

The Identity test proves the migration set applies and that one round trip works. It does not touch the
branches listed as "no" above.

## 4. What to add, in the order that finds the most first

Each item below is a test that would fail on one backend and not the other if the branch is wrong. They are
ordered by how much of the branch surface each one covers.

1. **`router_logs` round trip on PostgreSQL.** Covers the timestamp convention, `phs(is_postgres, 32)` for a
   32-column insert, and the `EXTRACT(EPOCH FROM ...)` expression through the time-window query. One test
   exercises an insert, a read, and an aggregation.
2. **`validate_token_and_get_info` on PostgreSQL.** Covers `"group"` quoting and the `$1` placeholder in
   one query, plus the `LEFT JOIN` on a second backend. The `LEFT JOIN` result is the part worth checking,
   because a backend difference there would silently turn authentication into a join failure.
3. **`ChannelProviderModel::create` on PostgreSQL.** Covers `RETURNING id`, which is a different statement
   from SQLite's, and the `"type"`/`"group"` quoting in an insert.
4. **`ChannelProtocolConfigModel::upsert` on PostgreSQL.** Covers `is_default = FALSE`, the
   `ON CONFLICT(channel_type, api_version) DO UPDATE` syntax, and `EXTRACT(EPOCH FROM NOW())` inside the
   column default.
5. **`RouterVideoTaskModel::save` on PostgreSQL.** Covers `ON CONFLICT (task_id) DO NOTHING` against
   SQLite's `INSERT OR IGNORE -- the two are not the same statement.

## 5. What a static check cannot settle, and what was checked statically anyway

Two properties of a SQL string can be checked without a server, and both were:

- **PostgreSQL placeholder numbering.** sqlx binds by position, so `$1..$n` must be consecutive with no gap
  or repeat. A static pass over the two crates reported no problems -- but the control showed it could only
  see four placeholder-bearing lines, because most placeholders are produced by `phs()` at runtime rather
  than written in the source. **So that clean result proves very little**, and it is recorded as such rather
  than presented as evidence.
- **Identifier quoting.** `` `group` `` must not appear in a PostgreSQL branch and `"group"` must not appear
  in a SQLite one. The audit found both used correctly.

Neither check can tell whether a query returns what its caller expects, which is why section 4 is a list of
tests rather than a list of greps.

## 6. Prerequisite

The tests in section 4 need a PostgreSQL server. `ci-integration.yml` provides one and sets
`BURNCLOUD_TEST_POSTGRES_URL`; the pattern to follow is `crates/identity/database-user/tests/postgres_round_trip.rs`,
which creates its own database per test and drops it afterwards, and skips with a printed reason when the
variable is unset.

Adding them before that job exists would mean writing tests that cannot run anywhere, which is the failure
mode this repository has already hit twice: the `database-router` suite that could not open its database,
and the identity tests that no workflow triggered.
