---
doc_id: agent.overview
doc_type: agent-protocol
truth: normative
status: active
---

# BurnCloud Agent Docs

BurnCloud Agent Docs define how autonomous software-engineering agents understand, change, verify, and report work in this repository.

They are not a repository encyclopedia. They are an execution system that routes an agent from a user-visible behavior to the minimum authoritative context required to work safely.

## Operating model

```text
User Task
   |
   v
Task Contract
   |
   v
AGENTS.md (Constitution + Router)
   |
   +----------------------+----------------------+
   |                      |                      |
   v                      v                      v
Domain Contract       Runtime / Contract      Playbook
   |                      |                      |
   +----------------------+----------------------+
                          |
                          v
                     Invariants
                          |
                          v
                      Source Code
                          |
                          v
                        Change
                          |
                          v
                    Verification
                          |
                          v
                   Invariant Check
                          |
                          v
                    Evidence Report
```

## Documentation layers

### 1. Constitution and routing

- `AGENTS.md` — highest repository-level agent rules and document router.
- `START_HERE.md` — required execution loop.
- `TASK_ROUTER.md` — recurring behavior -> source/test starting points.
- `TASK_CONTRACT.md` — minimum contract before non-trivial edits.

### 2. Domain contracts

`domains/` defines how domain documents must express responsibility, boundaries, sources of truth, side effects, invariants, risks, and verification.

Domain docs must be source-audited before they claim implementation facts. Do not manufacture a domain document merely to fill a directory.

### 3. Runtime, architecture, and contracts

Existing repository trees remain authoritative navigation/evidence layers:

- `docs/runtime/`
- `docs/architecture/`
- `docs/contracts/`

Agent Docs route into those trees; they do not duplicate them.

### 4. Invariants

- `INVARIANTS.md` contains current source-derived engineering invariants.
- `INVARIANT_STANDARD.md` defines how invariants are identified, named, reviewed, and verified.

### 5. Playbooks

`playbooks/` defines task-specific execution protocols such as bug fixes, features, refactors, provider integrations, and database changes.

A playbook is about **how to work**, not about current business facts.

### 6. Verification

- `TEST_MATRIX.md` maps areas to concrete repository tests.
- `verification/VERIFICATION_STANDARD.md` defines verification levels.
- `verification/DEFINITION_OF_DONE.md` defines when an agent may claim completion.

## Progressive disclosure

Do not load the entire documentation tree by default.

Use this path:

```text
Task
 -> AGENTS.md
 -> START_HERE.md
 -> TASK_ROUTER.md
 -> relevant domain/runtime/contract docs
 -> relevant invariants
 -> relevant source/tests
```

Only open additional documents when the current task crosses that boundary.

## Evidence vocabulary

Use these labels consistently:

- **STATIC CONFIRMED** — directly visible in current source/tests.
- **DYNAMIC** — runtime configuration, trait/adaptor/provider/channel selection, environment, data, or state controls the next target.
- **INFERRED** — plausible but not fully proven by inspected evidence.
- **UNKNOWN** — evidence has not yet established the fact.
- **RUNTIME VERIFIED** — directly observed in an executed runtime/integration/E2E path.

Do not turn DYNAMIC, INFERRED, or UNKNOWN into fixed architecture statements.

## Generated vs curated documentation

Prefer generation for facts that can be reliably extracted from source, such as:

- route inventories;
- symbol/package indexes;
- call/reference maps;
- provider registries;
- test indexes.

Human/agent-curated docs should primarily preserve what static extraction cannot safely infer:

- responsibility and ownership;
- business semantics;
- architectural boundaries;
- invariants;
- risk;
- change protocol;
- verification requirements;
- rationale.

## Maintenance loop

When source behavior changes, ask:

```text
Did ownership change?
Did a runtime path change?
Did an invariant change?
Did verification ownership change?
Did a contract/architecture fact change?
```

Update only the documents whose declared truth changed. Avoid documentation churn for refactors that preserve all externally relevant truths.
