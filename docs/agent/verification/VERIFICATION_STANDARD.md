---
doc_id: agent.verification-standard
doc_type: verification-guide
truth: normative
status: active
---

# Verification Standard

Verification is risk-based. Passing compilation proves only compilation; it does not prove the requested runtime behavior, regression safety, or invariant preservation.

Use `../TEST_MATRIX.md` for current repository-specific test ownership.

## Three proof questions

Every non-trivial change should answer:

1. **Functional:** does the requested behavior work?
2. **Regression:** did adjacent existing behavior remain intact?
3. **Invariant:** do relevant system truths still hold?

## Verification levels

### V0 — Inspection

Static inspection only.

Typical use: pure prose/docs changes with no executable behavior change.

### V1 — Build / static checks

Examples:

- formatting;
- type checking / compilation;
- linting;
- deterministic static validation scripts.

V1 does not by itself prove business behavior.

### V2 — Targeted behavior test

Run the closest unit/package/component test proving the changed branch or behavior.

### V3 — Regression verification

Run targeted tests plus adjacent suites protecting neighboring behavior and affected interfaces.

### V4 — Runtime / integration verification

Exercise the real multi-component execution path or integration boundary.

### V5 — End-to-end verification

Exercise the complete user/operator flow from its external entry through relevant side effects and response.

## Risk guidance

| Change area | Typical minimum |
|---|---|
| Documentation only | V0 |
| UI presentation only | V1-V2 |
| Local utility/internal logic | V2 |
| Handler/service behavior | V2-V3 |
| Provider/adaptor | V3 |
| Routing/failover | V3 |
| Billing/usage/quota | V3 |
| Authentication/authorization | V3 |
| Database migration/persistence semantics | V4 where feasible |
| Critical E2E data-plane behavior | V4-V5 where feasible |
| Provenance/security boundary | V4-V5 where feasible |

The table is a floor, not a ceiling. Raise the level when blast radius or uncertainty is higher.

## Unavailable checks

Never report an unavailable check as passed.

Report:

- what was attempted;
- why it could not run;
- what lower-level evidence was obtained instead;
- remaining risk.

## Documentation-only PRs

For Agent Docs changes, V0 is sufficient only when the diff is purely Markdown/prose and does not alter scripts/configuration/executable contracts. The required checks are then:

- inspect all changed paths;
- verify links/path references against the repository tree where practical;
- confirm no executable files changed;
- review the final diff for contradictions with current agent/runtime docs.
