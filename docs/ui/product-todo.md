---
doc_id: ui.product-todo
doc_type: product-execution-plan
truth: target-state-with-source-audit
status: draft
audited_against: c314bff9646f9113c9a58a818552fc80c77543a6
---

# BurnCloud Goal → TODO → Verification Plan v0.1

This backlog converts `product-text-prototype.md` into goal-driven execution. The unit of completion is a **verified product goal**, not a page, component, merged PR, or green CI run.

## 1. Status vocabulary

- `DEFINED` — target behavior and acceptance criteria are documented.
- `EXISTS` — the audited source directly contains the required primitive.
- `PARTIAL` — useful implementation exists, but the target contract is incomplete.
- `GAP` — no matching implementation was located in the baseline audit.
- `DESIGN REQUIRED` — architecture/security/provider design must be resolved first.
- `BLOCKED EXTERNAL` — completion depends on an external capability BurnCloud does not control.
- `VERIFIED DONE` — implementation, negative states, tests and goal-level verification all pass.

Never mark `VERIFIED DONE` merely because a PR merged or CI is green.

## 2. Current audited foundation

The current router log model already contains useful request evidence:

```text
request_id
user_id
upstream_id
model
HTTP status / latency
prompt/completion/cache/reasoning/etc token counts
cost breakdown
pricing_region
layer_decision
traffic_color
cost_status
error_type
```

These are **PARTIAL request evidence**, not a complete Trust Receipt.

The baseline repository audit did not locate a complete product implementation of:

```text
signed BurnCloud release manifest
release-artifact SHA-256 verification
BurnCloud release signer verification
canonical Trust Receipt request/response hashes
signed Request Trust Receipt
remote runtime attestation
complete provider-specific upstream evidence profiles
```

Re-audit the source whenever implementation changes; `GAP` is a backlog state, not a permanent claim.

## 3. Goal hierarchy

```text
G0 TRUST NORTH STAR
   Every customer request has an independently understandable,
   evidence-backed verification chain.

   ├── G1 API ROUTER
   │      A customer can reliably send a real AI request.
   │
   ├── G2 ROUTING ENGINE
   │      The route decision is stable, resilient and explainable.
   │
   └── G3 BUSINESS OS
          The operator can run customer, usage, cost and billing operations.

G1/G2/G3 inherit G0 truth and evidence rules.
```

## 4. G0 success test

For one selected real request, the intended verifier must be able to determine from product evidence:

1. local BurnCloud Client release identity;
2. whether the local artifact matches an official signed release;
3. remote server release claim;
4. whether runtime identity is attested or only declared;
5. privacy-safe request/customer/credential identity;
6. requested model;
7. route decision and eligible candidates;
8. selected upstream and retained provider evidence;
9. response/result/token/cost evidence;
10. which receipt fields are integrity-bound;
11. exactly why the chain passes, fails, or remains partial.

If an unsupported claim is rendered as generic `VERIFIED`, G0 fails.

## 5. P0 — Product semantics gate

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| P0.1 | Trust-first north star | `DEFINED` | independent verification is priority #1 |
| P0.2 | Claim vocabulary | `DEFINED` | declaration, hash match, signature, evidence and attestation are distinct |
| P0.3 | Request Trust Receipt target | `DEFINED` | target receipt sections + limitations documented |
| P0.4 | Machine-testable verification profiles | `DEFINED` | `verification-profiles.md` defines dimensions, profile requirements and downgrade/failure rules |
| P0.5 | Revalidate existing draft UI PRs | `PARTIAL` | #421 revalidated as PARTIAL; remaining draft UI work must be checked the same way |

**Gate:** no new bare `VERIFIED`, `FULL`, trust score, or confidence score may be introduced. Verification must name a profile.

## 6. P1 — Official release provenance

Goal: the customer independently verifies that the local BurnCloud Client matches an official signed release.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| P1.1 | Signed release manifest schema | `DESIGN REQUIRED` | canonical fields/serialization/versioning/signature envelope defined |
| P1.2 | Release signing algorithm + key lifecycle | `DESIGN REQUIRED` | public key distribution, rotation, revocation and compromise handling defined |
| P1.3 | Publish SHA-256 for supported artifacts | `GAP` | release workflow emits digest for every declared artifact |
| P1.4 | Sign official release manifest | `GAP` | protected release key signs canonical manifest |
| P1.5 | Bind release to git/source/build identity | `GAP` | manifest identifies the source/build the artifact represents |
| P1.6 | Client manifest verification | `GAP` | wrong signer/invalid signature fails closed |
| P1.7 | Client local artifact SHA-256 check | `GAP` | mutated artifact becomes mismatch, not verified |
| P1.8 | Negative tests | `GAP` | forged manifest/wrong key/changed artifact/unsupported schema explicitly fail |

### P1 final verification

Official package, mutated package and forged manifest must produce three distinguishable outcomes without operator interpretation.

## 7. P2 — Server identity and runtime proof

Goal: the client accurately states what it can prove about the remote server.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| P2.1 | Server build identity API | `DESIGN REQUIRED` | exposes version/commit/build claim without calling it attestation |
| P2.2 | Verify claimed release against official manifest | `GAP` | client can prove the claim references an official signed release |
| P2.3 | `DECLARED / NOT ATTESTED` UX | `GAP` | signed release claim never becomes runtime-verified by implication |
| P2.4 | Choose first attestation target + threat model | `DESIGN REQUIRED` | concrete TEE/TPM/cloud mechanism and accepted measurements defined |
| P2.5 | Implement attestation verifier | `GAP` | invalid chain/challenge/measurement fails closed |
| P2.6 | Bind attestation to receipt signer identity | `GAP` | attested workload controls/binds the key used for request receipts |
| P2.7 | Freshness/replay defense | `DESIGN REQUIRED` | nonce/challenge/expiry policy prevents stale proof reuse |
| P2.8 | Negative tests | `GAP` | wrong measurement/stale evidence/unsupported platform stay non-attested |

### P2 final verification

A server claiming the correct official version but supplying no accepted attestation must still display `NOT ATTESTED`.

## 8. P3 — Request Trust Receipt

Goal: every eligible routed request produces a stable, privacy-safe evidence object.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| P3.1 | Stable request identity | `EXISTS` | `request_id` persisted/queryable |
| P3.2 | Customer/credential attribution | `PARTIAL` | current identity primitives mapped to privacy-safe receipt semantics |
| P3.3 | Requested model | `EXISTS` | persisted model available |
| P3.4 | Selected upstream | `PARTIAL` | `upstream_id` mapped to explicit provider evidence identity |
| P3.5 | Routing decision | `PARTIAL` | `layer_decision` exists; candidate snapshot + final receipt semantics remain |
| P3.6 | Token/cost evidence | `PARTIAL` | existing detailed fields mapped to exact source/scope contract |
| P3.7 | Canonical request hash | `GAP` | canonicalization excludes secrets and is request-class aware |
| P3.8 | Canonical response hash | `GAP` | streaming/non-streaming rules separately specified |
| P3.9 | Versioned receipt schema | `DESIGN REQUIRED` | backward compatibility policy exists |
| P3.10 | Persist receipt hash/key/signature/profile | `GAP` | proof fields queryable with request |
| P3.11 | Sign receipt | `GAP` | post-sign mutation fails VP1 integrity verification |
| P3.12 | Privacy/security export policy | `DESIGN REQUIRED` | bearer/provider secrets cannot enter exported artifact |
| P3.13 | Export receipt | `GAP` | customer/auditor can obtain stable artifact |
| P3.14 | Offline/client verifier | `GAP` | verifier does not need to trust Console rendering |

### P3 final verification

Changing an integrity-covered field after signing must produce `FAIL`. Missing optional provider evidence must downgrade only the relevant profile.

## 9. P4 — Provider-specific upstream evidence

Goal: BurnCloud shows the strongest evidence actually available for each upstream path, never fabricated parity.

Common adapter target:

```text
provider_family
endpoint_identity
region
resolved_provider_model
privacy_safe_account_alias
provider_request_id / response metadata
transport evidence
provider-specific evidence fields
coverage / limitation reason
```

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| P4.1 | Generic evidence adapter interface | `DESIGN REQUIRED` | typed evidence + limitation reasons returned |
| P4.2 | AWS Bedrock profile | `GAP` | only evidence available on real AWS path is claimed |
| P4.3 | Anthropic profile | `GAP` | only Anthropic-supported metadata is claimed |
| P4.4 | Google/Gemini profile | `GAP` | provider-specific evidence mapped truthfully |
| P4.5 | Azure/OpenAI profile | `GAP` | provider-specific evidence mapped truthfully |
| P4.6 | Redaction contract | `DESIGN REQUIRED` | credentials/authorization material excluded |
| P4.7 | Unsupported provider state | `GAP` | profile becomes UNKNOWN/LIMITED, not fake verified |
| P4.8 | Negative tests | `GAP` | missing metadata downgrades relevant proof only |

### P4 final verification

An AWS-labelled channel with only `upstream_id` and no provider-native evidence must never be described as cryptographically verified AWS execution.

## 10. P5 — BurnCloud Client trust UX

Goal: a non-expert customer can understand verification without knowing BurnCloud internals.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| P5.1 | Local release verification screen | `GAP` | signature/SHA-256/version/commit + failure reasons visible |
| P5.2 | Connected server verification | `GAP` | release claim separated from runtime attestation |
| P5.3 | Requests trust profile column | `GAP` | named profile, not subjective score |
| P5.4 | Request Trust Receipt detail | `GAP` | product prototype sections populated from real evidence |
| P5.5 | `Why partial?` explanation | `GAP` | missing/failed dimensions listed |
| P5.6 | Export + Verify actions | `GAP` | independent verification path works |
| P5.7 | Verification explanation | `GAP` | UI says what each proof does and does not prove |
| P5.8 | Clarity/accessibility test | `GAP` | test user distinguishes declared vs signed vs attested |

## 11. G1 — API Router

Goal: from a clean supported environment, an operator can produce a successful customer-callable AI request through BurnCloud.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| G1.1 | Provider configuration | `PARTIAL` | truthful state/error UX for supported provider |
| G1.2 | Derived model availability | `PARTIAL` | configured/available/redundant distinct |
| G1.3 | Route/failover configuration | `PARTIAL` | operator understands how traffic can be served |
| G1.4 | Customer/credential ownership | `PARTIAL` | safe account-owned bearer lifecycle |
| G1.5 | Real Playground request | `PARTIAL` | successful test persists request evidence |
| G1.6 | Clean-environment E2E | `GAP` | one scenario proves setup → routed request → evidence |

G1 cannot be `VERIFIED DONE` until its request enters the G0 evidence chain.

## 12. G2 — Routing Engine

Goal: for one request, the operator/customer can understand candidates, policy, selected upstream and fallback from persisted decision-time facts.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| G2.1 | Candidate model | `PARTIAL` | provider/model/group primitives exist |
| G2.2 | Persist eligible candidate snapshot | `GAP` | no reconstruction from current config required |
| G2.3 | Persist decision reason | `PARTIAL` | `layer_decision` becomes defined evidence |
| G2.4 | Persist failover chain | `PARTIAL` | each fallback step explicit enough to explain |
| G2.5 | Health/capacity decision evidence | `DESIGN REQUIRED` | only actual router inputs retained/explained |
| G2.6 | Cost-aware decision evidence | `DESIGN REQUIRED` | actual cost source/policy explainable if used |
| G2.7 | Routing explanation UI | `GAP` | answers `why this upstream?` from receipt facts |
| G2.8 | Explanation invariants tests | `PARTIAL` | existing router tests extended to evidence contract |

## 13. G3 — Business OS

Goal: manage customer → credential → usage → upstream cost → customer charge → margin/settlement from authoritative sources.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| G3.1 | Customer lifecycle/funding | `PARTIAL` | existing capability mapped to explicit business semantics |
| G3.2 | Credential lifecycle | `PARTIAL` | ownership and secret handling safe/auditable |
| G3.3 | Usage scope contract | `PARTIAL` | account/environment scope explicit |
| G3.4 | Billing source contract | `PARTIAL` | not reconstructed from bounded request samples |
| G3.5 | Upstream cost evidence | `PARTIAL` | cost/pricing fields mapped to authoritative source |
| G3.6 | Customer charge model | `DESIGN REQUIRED` | selling price/discount separate from upstream cost |
| G3.7 | Margin model | `GAP` | revenue minus cost explainable by period/customer/model/request |
| G3.8 | Settlement workflow | `DESIGN REQUIRED` | prepaid/postpaid/receivable authoritative ledger defined |
| G3.9 | Business → request drilldown | `GAP` | aggregate numbers reach supporting evidence where permitted |

## 14. UI migration order after semantics freeze

```text
1. Overview                 trust conclusion + next action
2. Providers                truthful upstream identity/evidence capability
3. Models                   availability without proof overclaim
4. Routes                   candidate/decision explanation
5. Playground               create one real verifiable request
6. Logs → Requests          request evidence becomes primary object
7. Request Trust Receipt    complete per-request chain
8. Verification             release/runtime/profile explanation
9. Customers / API Keys     business identity → credential
10. Usage / Billing         explicit authoritative scope
11. Guardrails/Team/Settings governance/runtime boundaries
12. Cross-page visual polish
```

Do not treat this as a page-completion checklist; every item must close goal/TODO acceptance criteria.

## 15. Verification matrix

| Goal | User question | Required evidence | Final verification |
| --- | --- | --- | --- |
| G0 Trust | Can I verify what happened without trusting the operator? | signed release + truthful runtime state + receipt + upstream evidence | one real request independently evaluated including limitations |
| G1 Router | Can BurnCloud reliably serve my API request? | supply + route + credential + successful request | clean-environment E2E succeeds and persists evidence |
| G2 Routing | Why did this request use this upstream? | candidate snapshot + decision + failover + selected upstream evidence | explanation uses persisted request-time facts |
| G3 Business | Can I run the token/API business from authoritative data? | customer + credential + usage + cost + charge/settlement | totals reconcile to authoritative sources and request evidence where applicable |

## 16. Per-PR completion template

Every implementation PR that advances a TODO must answer:

```text
TODO ID:
Goal:
User question:
Claim added/changed:
Source of truth:
Scope:
Verification profile/evidence level:
Unknown/loading/error behavior:
Negative test:
Cross-page handoff:
Security/privacy impact:
Automated validation:
Manual/visual validation:
Goal verification result: PASS / PARTIAL / FAIL
```

`PASS` here means that PR's declared goal acceptance passed. It does not automatically close the parent product goal.

## 17. Near-term execution order

```text
1. Agree product spine (#420)
2. Agree Product Text Prototype + TODO (#422)
3. Agree verification profiles (P0.4)        ← defined in this PR
4. Design signed release manifest/key lifecycle (P1.1/P1.2)
5. Implement SHA-256 + official release signatures (P1)
6. Design Trust Receipt schema/canonical hashing (P3)
7. Implement first provider evidence profile (P4)
8. Build Client Trust Receipt / Verification UX (P5)
9. Revalidate Overview/current pages against G0-G3
10. Resume broad page migration/pixel polish only after semantic convergence
```

Runtime attestation is a separate harder architecture track. Until P2 is implemented, the product must truthfully show `NOT ATTESTED`; other evidence work does not need to wait for it.
