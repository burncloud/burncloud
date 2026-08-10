---
doc_id: standard.database
doc_type: engineering-standard
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Database Standards

## Current database support

The workspace SQLx dependency enables both SQLite and PostgreSQL. Database code is split between `crates/database` and domain child crates under `crates/database/crates/`.

Do not use an old schema document as authority. Inspect migrations/schema initialization and the current domain database code for the table/column being changed.

## Placeholder portability

`crates/database/src/placeholder.rs` is the current cross-database placeholder utility:

- `ph(is_postgres, n)` → `?` or `$n`,
- `phs(is_postgres, count)` → a list of dialect-correct placeholders,
- `adapt_sql(is_postgres, sql)` → converts `?` placeholders to PostgreSQL numbering.

For SQL intended to support both engines, use the shared mechanism rather than hard-coding only one dialect.

## Change procedure

For persistence changes:

1. identify the owning domain database crate;
2. verify current schema/init/migration behavior;
3. trace service/server/router callers;
4. check SQLite/PostgreSQL differences;
5. run domain tests and the affected user-flow integration tests;
6. update current contracts/invariants if observable state semantics change.

## No mixed future/current schema docs

Do not add planned tables/columns to a current-schema reference before implementation. Future schema design belongs in a GitHub Issue/PR until code/migrations exist.
