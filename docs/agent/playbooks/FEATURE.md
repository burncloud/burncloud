---
doc_id: agent.playbook.feature
doc_type: agent-playbook
truth: normative
status: active
---

# Feature Playbook

Use this for new observable behavior.

## Workflow

```text
UNDERSTAND REQUEST
 -> IDENTIFY DOMAIN
 -> FIND EXISTING PATTERN
 -> DEFINE EXPECTED BEHAVIOR
 -> DEFINE INVARIANTS / COMPATIBILITY
 -> DESIGN MINIMUM PATH
 -> IMPLEMENT
 -> VERIFY NEW BEHAVIOR
 -> REGRESSION VERIFY
 -> UPDATE CHANGED TRUTHS IN DOCS
 -> REPORT
```

## Before implementation

Establish:

- user/operator outcome;
- real entry point;
- ownership domain;
- existing nearby pattern that should be reused;
- public API/data/config compatibility requirements;
- persistence/external/billing/auth/routing impacts;
- failure behavior;
- acceptance criteria.

Do not design from a hypothetical architecture if existing source already provides an extension point.

## Implementation rule

Prefer the minimum complete vertical slice that produces the requested behavior. Avoid constructing unused abstractions for imagined future features.

## Verification

Verify:

1. requested behavior;
2. invalid/error paths that are part of the feature contract;
3. adjacent existing behavior;
4. affected invariants;
5. documentation whose declared truth changed.

If the feature introduces a recurring ownership boundary or recurring engineering task, consider adding an audited Domain Contract or `TASK_ROUTER.md` entry. Do not add routing/docs merely because new files exist.
