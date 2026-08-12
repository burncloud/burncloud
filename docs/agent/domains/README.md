---
doc_id: agent.domain-contract-standard
doc_type: engineering-standard
truth: normative
status: active
---

# Domain Contracts

A Domain Contract describes a stable ownership boundary that agents repeatedly need to reason about.

Domain docs are not package summaries. A directory is not automatically a domain, and a domain may span multiple crates/modules.

## When to create a domain document

Create one only when all are true:

1. the behavior recurs across engineering tasks;
2. ownership can be established from current source/contracts;
3. important invariants or side effects belong to the area;
4. a dedicated document reduces repeated repository-wide search.

Do not create placeholder domain facts that have not been audited.

## Required structure

```md
---
doc_id: agent.domain.<name>
doc_type: domain-contract
truth: source-derived
status: active
audited_against: <commit-sha>
---

# <Domain> Domain Contract

## Responsibility
What this domain owns.

## Does not own
Boundaries intentionally owned elsewhere.

## Source of truth
Current source/schema/configuration that defines behavior.

## Entry points
Real routes/events/callers entering the domain.

## Core components
Important symbols/modules after source verification.

## Inputs
Important inputs.

## Outputs
Important outputs.

## Side effects
Persistence, external calls, billing, logs, state, etc.

## Dependencies
Other stable domains/systems used.

## Invariants
Relevant `INV-*` IDs.

## Runtime / contract docs
Links to existing `docs/runtime/`, `docs/contracts/`, or `docs/architecture/` evidence maps.

## Modification risks
Likely blast radius and dangerous semantic changes.

## Required verification
Minimum verification for changes in this domain.

## Related tests
Stable tests/suites to inspect.

## Dynamic boundaries
Runtime-selected provider/channel/trait/config edges.

## Known unknowns
Facts intentionally not claimed because evidence is incomplete.
```

## Initial domain candidates

Current `TASK_ROUTER.md` already identifies recurring engineering areas that may justify audited domain contracts over time:

- router / channel selection;
- provider execution/adapters;
- billing / usage / quota;
- authentication / authorization;
- channel management;
- API token/key management;
- database behavior;
- Console/UI workflows.

This list is a documentation roadmap, not a claim that each boundary has already been formally audited.

## Boundary rule

A good Domain Contract helps answer:

> “Is this the right layer to solve the problem?”

It should make responsibility and non-responsibility equally clear so an agent does not solve a routing problem in billing, an auth problem in UI, or a provider-conversion problem by changing unrelated public API semantics.
