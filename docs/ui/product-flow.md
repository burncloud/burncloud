---
doc_id: ui.product-flow
doc_type: product-architecture-standard
truth: source-derived
status: active
audited_against: c314bff9646f9113c9a58a818552fc80c77543a6
---

# BurnCloud Console Product Flow

This document is the product-architecture contract for the routed BurnCloud Console.

It answers a different question from `pages.md`:

- `pages.md` defines **how to polish and verify one page**;
- this document defines **why each page exists, what it owns, what it must not own, and how pages hand work to one another**.

When a page-level design conflicts with this flow, fix the page rather than inventing another parallel workflow.

## 1. Core rule

The Console has one operating loop:

```text
Overview
   ↓
Supply: Providers → Models → Routes
   ↓
Access: Customers → API Keys
   ↓
Verify: Playground
   ↓
Observe: Logs → Evaluation → Billing
   ↓
Govern: Guardrails → Team → Settings
```

This is a **responsibility flow**, not a claim that every installation must complete every page before any request can run.

For example, Customers is the business-account provisioning surface, but an administrator may already have an account that can own a credential. Therefore Customers must precede API Keys in the product model without being treated as a universal environment-readiness prerequisite.

## 2. Navigation contract

The operator navigation is grouped by user intent, not by implementation package:

1. **Workspace**
   - Overview
2. **Supply**
   - Providers
   - Models
   - Routes
3. **Access**
   - Customers
   - API Keys
4. **Verify & Observe**
   - Playground
   - Logs
   - Evaluation
   - Billing
5. **Govern**
   - Guardrails
   - Team
   - Settings

Do not create a second navigation order inside Overview, empty states, onboarding copy, or page CTAs.

A page may skip an already-satisfied step, but it must not contradict the canonical sequence.

## 3. One page, one primary question

| Page | Primary question | Owns | Must not become |
| --- | --- | --- | --- |
| Overview | Can I operate this environment, what is unverified, and what should I do next? | Cross-domain conclusion and next action | A second Providers, Logs, Billing, or Settings page |
| Providers | What upstream supply is configured and usable? | Provider/channel configuration and lifecycle | A model catalog or route-health dashboard |
| Models | Which model IDs are exposed by configured providers? | Derived model availability and provider coverage | Independent model CRUD when the backend has none |
| Routes | How can model traffic be served and fail over? | Derived group/model route coverage and resilience | Provider editing duplicated inline |
| Customers | Which business accounts can consume BurnCloud and what wallet state do they have? | Business account creation and funding | Staff/admin directory |
| API Keys | Which account-owned credentials can call the router and under what limits? | Credential lifecycle, spend limit, network restriction | Customer creation or bearer-secret redisclosure |
| Playground | Does a controlled request work through the real BurnCloud path? | End-to-end verification request and current test evidence | A chat product or alternate inference path |
| Logs | What happened to an individual routed request? | Request-level operational evidence | Billing ledger or synthetic evaluation score |
| Evaluation | What does the bounded observed request sample indicate? | Aggregated operational observations | Configured redundancy truth or model-quality benchmark |
| Billing | What did the authenticated account spend and use? | Account-scoped financial usage | Environment-wide spend inferred from Logs |
| Guardrails | What traffic protections are persisted, what HTTP risk signals exist, and is emergency stop needed? | Protection policy and emergency routing stop | Threat-intelligence or IDS claims derived from HTTP errors |
| Team | Who currently satisfies Console administrator authorization? | Administrator inventory and session authority | Fake invite/role CRUD without backend endpoints |
| Settings | What environment/runtime/cache state can be inspected or maintained? | Runtime/environment diagnostics and supported maintenance | General business configuration or duplicate Overview health |

## 4. Scope contract

Every important conclusion must make its scope obvious.

### Environment-scoped

- provider availability
- model exposure
- route coverage
- router request logs
- operational evaluation sample
- guardrail policy and HTTP risk signals
- runtime/cache state
- Console administrator inventory

### Account-scoped

- authenticated-user usage
- authenticated-user billing
- account-owned API credential attribution

### Business-account inventory

- Customers
- wallet balances/funding

Never silently combine account-scoped money with environment-scoped traffic.

## 5. Evidence contract

Across every page, use the same evidence ladder:

```text
UNKNOWN → CONFIGURED → AVAILABLE → VERIFIED → OBSERVED
```

Definitions:

- **UNKNOWN** — the source is loading, failed, or not queried. Never render zero as a substitute.
- **CONFIGURED** — persisted configuration exists. This does not prove runtime reachability.
- **AVAILABLE** — the current backend/runtime says the capability can be used.
- **VERIFIED** — a direct operation proved the path, for example a successful Playground request.
- **OBSERVED** — evidence exists in request logs or another bounded operational sample. Observation is not configuration truth.

Additional rules:

- configured is not verified;
- observed diversity is not configured redundancy;
- HTTP errors are not automatically security threats;
- a status metadata field is not an enforcement guarantee unless the request/auth path proves it;
- a management identifier is not a bearer credential;
- unknown is not zero.

## 6. Handoff contract

Pages should pass the user forward instead of duplicating the next page.

### Supply

- Providers creates the upstream facts.
- Models derives model exposure from provider facts.
- Routes derives route/failover coverage from provider + model facts.
- A broken model/route sends the user back to Providers for repair rather than adding fake CRUD.

### Access

- Customers creates/funds a business identity.
- API Keys assigns router access to a verified owner identity.
- API Keys does not create Customers as a side effect.

### Verification and observation

- Playground tests the real route.
- Successful or failed Playground tests hand off to Logs for persisted request evidence.
- Logs hands aggregate investigation to Evaluation.
- Billing is sourced from the billing contract, not reconstructed from the bounded Logs sample.

### Governance

- Guardrails links request-level diagnosis to Logs instead of reproducing the Logs detail surface.
- Team reflects the real Console authorization boundary and stays read-only until role-management APIs exist.
- Settings handles runtime/cache maintenance and does not duplicate business or routing configuration.

## 7. Overview contract

Overview is the only cross-domain page, so it has the strictest boundary.

It should show, in order:

1. **Primary conclusion** — what is the most important current state?
2. **Evidence level** — unknown/configured/available/verified/observed.
3. **Next action** — one primary CTA to the owning page.
4. **Compact supporting facts** — only enough information to justify the conclusion.
5. **Deep links** — navigate to the owner page for detail or mutation.

Overview must not provide provider editing, credential lifecycle, customer funding, guardrail editing, cache maintenance, or request-detail diagnosis.

## 8. Standard page skeleton

Every primary Console page should converge on the same information hierarchy:

```text
Page title + one-sentence responsibility
Primary conclusion / state
Primary action + one secondary investigation action
Key facts (only facts owned by this page)
Main workflow / inventory
Evidence / explanation / empty state
Danger or maintenance operations last and visually separated
```

A page is incomplete if the user cannot answer these three questions within the first screen:

1. Why am I here?
2. What is the current state?
3. What should I do next?

## 9. Migration order

Do not rewrite the whole Console in one PR. Migrate in this order:

### Phase 1 — Product spine

- canonical navigation groups/order
- Overview uses the same sequence and vocabulary
- cross-page CTA destinations match ownership

### Phase 2 — Supply

- Providers
- Models
- Routes

Goal: one coherent configuration-to-resilience story.

### Phase 3 — Access

- Customers
- API Keys

Goal: identity → wallet → credential ownership is explicit.

### Phase 4 — Verify and observe

- Playground
- Logs
- Evaluation
- Billing

Goal: test → request evidence → aggregate observation → account financial truth.

### Phase 5 — Govern

- Guardrails
- Team
- Settings

Goal: policy, authorization, runtime maintenance remain separate responsibilities.

### Phase 6 — Cross-page visual acceptance

Only after responsibilities and state semantics are aligned:

- spacing/density consistency
- table/action consistency
- responsive behavior
- 1440×900 end-to-end screenshot/click-path review

## 10. PR acceptance checklist

Every UI PR must state:

- which page responsibility it changes;
- which upstream page supplies its facts;
- which downstream page receives its next action;
- whether each displayed fact is environment, account, or business-account scoped;
- which evidence level supports each major status;
- which neighboring responsibility was intentionally *not* duplicated;
- loading/error/empty/danger behavior;
- executable UI checks and relevant Rust/browser validation.

If a PR cannot answer those points, the product logic is not sufficiently defined yet.
