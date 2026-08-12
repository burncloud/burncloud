---
doc_id: agent.invariant-standard
doc_type: engineering-standard
truth: normative
status: active
---

# Invariant Standard

An invariant is a high-value truth that must remain valid across legitimate implementation changes unless the product/architecture contract intentionally changes.

`INVARIANTS.md` is the current inventory. This document defines how that inventory is maintained.

## What qualifies as an invariant

A useful invariant is:

- stable across ordinary refactors;
- meaningful to correctness, safety, compatibility, or system semantics;
- source-derived or contract-backed;
- verifiable through code/tests/runtime evidence;
- important enough that a future agent should check it before changing the area.

Do not convert incidental implementation detail into an invariant.

## Naming

Use stable IDs:

`INV-<DOMAIN>-NNN`

Examples:

- `INV-AUTH-001`
- `INV-ROUTER-001`
- `INV-BILLING-001`
- `INV-PROVIDER-001`
- `INV-TRACE-001`

IDs should not be recycled after removal.

## Risk levels

- **Critical** — violation can cause security boundary failure, incorrect charging, tenant/user isolation failure, false provenance, secret exposure, or destructive persistence errors.
- **High** — violation can materially change routing/provider execution, quota/usage, public API semantics, retry/streaming behavior, or migrations.
- **Medium** — violation can break a bounded feature or operational contract.
- **Low** — stable engineering convention with limited runtime blast radius.

## Recommended invariant format

```md
### INV-DOMAIN-001 — Short name

**Risk:** High

**Rule:**
A precise statement that should remain true.

**Why:**
Why the rule matters.

**Applies to:**
- affected runtime modes / routes / providers / data paths

**Evidence:**
- `path/to/file.rs :: SymbolName`
- `path/to/test.rs :: test_name`

**Verification:**
How an agent can prove the invariant after a change.

**Dynamic boundaries:**
Any runtime-selected edge that must not be represented as static.
```

## Candidate invariant process

When an agent discovers a potentially important rule:

1. verify that it is not merely a local implementation accident;
2. find stable evidence;
3. assess its blast radius;
4. add a stable ID only if future changes should explicitly preserve/check it;
5. add/identify verification where possible.

If evidence is incomplete, document the claim elsewhere as INFERRED/UNKNOWN instead of promoting it to a verified invariant.

## Changing an invariant

An intentional invariant change requires coordinated review of:

- implementation;
- tests;
- runtime/architecture/contracts that describe the behavior;
- `INVARIANTS.md`;
- downstream compatibility/risk.

Do not edit an invariant merely to make a conflicting implementation appear compliant.

## Non-invariants

Keep explicit non-invariants in `INVARIANTS.md` when an outdated architectural belief is likely to mislead future agents. This is especially valuable when historical docs or familiar patterns strongly suggest behavior that current source does not implement.
