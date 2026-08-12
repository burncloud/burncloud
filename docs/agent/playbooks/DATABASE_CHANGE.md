---
doc_id: agent.playbook.database-change
doc_type: agent-playbook
truth: normative
status: active
---

# Database Change Playbook

Use this for schema, migration, cross-dialect SQL, persistence semantics, or database API changes.

## Before editing

Inspect:

- current schema and migrations;
- database abstraction used by the affected crate;
- existing rows/backfill implications;
- NULL/default semantics;
- indexes/constraints;
- transactions and failure atomicity;
- callers of changed persistence APIs;
- SQLite/PostgreSQL compatibility where the repository supports both;
- rollback/backward compatibility risk.

Do not modify only an ORM/model structure if the real database schema/migration must also change.

## Cross-dialect rule

`INVARIANTS.md` currently documents the repository's placeholder abstraction for SQLite/PostgreSQL. Do not hard-code one placeholder dialect in code expected to support both databases without verifying the intended scope.

## Workflow

```text
CONFIRM CURRENT SCHEMA
 -> TRACE READ/WRITE CALLERS
 -> DEFINE DATA COMPATIBILITY
 -> DESIGN MIGRATION / API CHANGE
 -> IMPLEMENT
 -> VERIFY BOTH APPLICATION AND DATA BEHAVIOR
 -> REGRESSION VERIFY
 -> REVIEW ROLLBACK / FAILURE PATH
```

## Verification

Depending on the change, verify:

- migration application;
- reads and writes with existing/new data;
- defaults/NULL behavior;
- constraints/index behavior;
- transaction rollback/error paths;
- affected service/API flow;
- both supported SQL dialect paths when applicable.

Schema changes generally require higher verification than a local query refactor. Use `../verification/VERIFICATION_STANDARD.md`.
