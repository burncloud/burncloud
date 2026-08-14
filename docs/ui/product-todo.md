---
doc_id: ui.product-todo
doc_type: product-execution-plan
truth: target-state-with-source-audit
status: draft
audited_against: c314bff9646f9113c9a58a818552fc80c77543a6
---

# BurnCloud Goal → TODO → Verification Plan v0.1

This document converts `product-text-prototype.md` into an executable product backlog.

The unit of completion is a **verified product goal**, not a page, component, or merged PR.

## 1. Status vocabulary

Use these statuses until a task has explicit implementation evidence:

- `DEFINED` — target behavior and acceptance criteria are documented.
- `EXISTS` — the current audited source directly contains the required primitive.
- `PARTIAL` — useful implementation exists, but the target product contract is incomplete.
- `GAP` — no matching implementation was located in the baseline source audit; this is a backlog item, not a proof of impossibility.
- `DESIGN REQUIRED` — architecture/security/provider design must be resolved before implementation.
- `BLOCKED EXTERNAL` — completion depends on a provider/platform capability BurnCloud does not control.
- `VERIFIED DONE` — implementation, negative states, tests and goal-level verification all pass.

Do not mark a task `VERIFIED DONE` because a PR merged or CI is green.

## 2. Current audited foundation

The baseline audit found useful request-observability primitives in the current router log model, including:

- `request_id`;
- `user_id`;
- `upstream_id`;
- `model`;
- HTTP status and latency;
- detailed token/cost fields;
- `pricing_region`;
- `layer_decision`;
- `traffic_color`;
- `cost_status`;
- `error_type`.

These fields are **PARTIAL request evidence**, not a complete Trust Receipt.

The baseline repository search did not locate a complete implementation of:

- signed BurnCloud release manifests;
- release artifact SHA-256 verification as a product feature;
- BurnCloud release signer/public-key verification;
- canonical request/response hashes for Trust Receipts;
- signed Request Trust Receipts;
- remote runtime attestation;
- provider-specific upstream evidence profiles sufficient for the target trust UI.

These findings are the starting backlog, not permanent assumptions. Re-audit source whenever implementation changes.

## 3. Goal hierarchy

```text
G0  TRUST NORTH STAR
    Every customer request has an independently understandable,
    evidence-backed verification chain.

    ├── G1  API ROUTER
    │       A customer can reliably send a real AI request.
    │
    ├── G2  ROUTING ENGINE
    │       The route decision is stable, resilient and explainable.
    │
    └── G3  BUSINESS OS
            The operator can run customer, usage, cost and billing operations.

All G1/G2/G3 claims inherit G0 truth and evidence rules.
```

## 4. G0 — Trust north star

### G0 success statement

For a selected routed request, the intended verifier can determine:

1. what BurnCloud Client release they are using;
2. whether that local release matches a signed official artifact;
3. what the remote BurnCloud Server claims to run;
4. whether remote runtime identity is independently attested or only declared;
5. who/what initiated the request using privacy-safe identities;
6. which model was requested;
7. how routing selected an upstream;
8. what actual upstream evidence was retained;
9. what response/result/tokens/cost were recorded;
10. which receipt fields are integrity-bound;
11. exactly why the verification result is full, partial, limited or unknown.

### G0 verification test

A reviewer must be able to take one test request and answer all eleven questions from product evidence without relying on a screenshot or an operator's verbal explanation.

If any unsupported claim is rendered as generic `VERIFIED`, G0 fails.

## 5. P0 — Freeze product semantics before more page polish

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| P0.1 | Define trust-first north star | `DEFINED` | Product prototype names independent verification as priority #1 |
| P0.2 | Define claim vocabulary | `DEFINED` | `UNKNOWN`, `DECLARED`, `HASH MATCH`, `SIGNATURE VERIFIED`, `EVIDENCED`, `RUNTIME ATTESTED`, `CHAIN VERIFIED` have non-overlapping meanings |
| P0.3 | Define Request Trust Receipt target | `DEFINED` | Receipt sections and proof limitations are documented |
| P0.4 | Define verification profiles | `DESIGN REQUIRED` | Exact requirements for `FULL`, `PARTIAL`, `LIMITED` (or final names) are machine-testable |
| P0.5 | Revalidate existing draft UI PRs | `GAP` | Every draft states which prototype goal it satisfies and removes conflicting semantics |

**Gate:** do not introduce new generic trust scores or `VERIFIED` badges before P0.4 is resolved.

## 6. P1 — Official release provenance

Goal: a customer can independently verify that the local BurnCloud Client package matches an official signed release.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| P1.1 | Specify signed release manifest schema | `DESIGN REQUIRED` | Canonical fields, serialization/canonicalization, versioning and signature envelope documented |
| P1.2 | Choose release signing algorithm/key lifecycle | `DESIGN REQUIRED` | Public verification key distribution, rotation, compromise and revocation behavior documented |
| P1.3 | Generate SHA-256 for supported artifacts | `GAP` | CI/release pipeline publishes digest for every declared artifact |
| P1.4 | Sign release manifest in official release workflow | `GAP` | Signature produced by protected release key and independently verifiable |
| P1.5 | Publish git commit/source identity | `GAP` | Manifest binds release to repository source/build identity |
| P1.6 | Implement client-side manifest verification | `GAP` | Invalid signature fails closed and displays reason |
| P1.7 | Implement local artifact SHA-256 verification | `GAP` | Mutated local artifact produces mismatch, never generic verified |
| P1.8 | Add release-provenance negative tests | `GAP` | Tampered manifest, wrong key, changed artifact and unsupported schema all fail explicitly |

### P1 goal verification

Given an official client package, a mutated package, and a forged manifest, the verifier must distinguish:

- valid signature + matching SHA-256;
- valid signature + mismatching artifact;
- invalid/unknown signer;
- unknown/unsupported proof.

## 7. P2 — Server identity and runtime proof

Goal: the client tells the truth about what it can prove about the remote server.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| P2.1 | Define server build identity API | `DESIGN REQUIRED` | Endpoint exposes version/commit/build identity without calling it attestation |
| P2.2 | Bind server claim to signed release manifest | `GAP` | Client can verify the claimed release exists in an official signed manifest |
| P2.3 | UI state `DECLARED / NOT ATTESTED` | `GAP` | Signed release claim is not rendered as remote-runtime verified |
| P2.4 | Select first remote-attestation deployment target | `DESIGN REQUIRED` | TEE/TPM/cloud mechanism, threat model and accepted measurements are explicit |
| P2.5 | Implement attestation verifier | `GAP` | Invalid measurement/challenge/certificate fails closed |
| P2.6 | Bind attestation to server release measurement | `GAP` | Accepted attestation proves a measurement covered by policy |
| P2.7 | Define attestation freshness/replay defense | `DESIGN REQUIRED` | Challenge/nonce/expiry behavior prevents stale proof reuse |
| P2.8 | Runtime-attestation negative tests | `GAP` | Wrong measurement, stale evidence and unsupported platform become non-attested states |

### P2 goal verification

A non-attested server that claims the correct official version must still display `NOT ATTESTED`.

This is a mandatory negative acceptance test.

## 8. P3 — Request Trust Receipt foundation

Goal: every eligible routed request can produce a stable, privacy-safe evidence object.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| P3.1 | Stable request identity | `EXISTS` | `request_id` is persisted and queryable |
| P3.2 | User/account attribution | `PARTIAL` | current `user_id` is usable; target privacy-safe customer/credential semantics are specified |
| P3.3 | Requested model | `EXISTS` | persisted request model available |
| P3.4 | Selected upstream identity | `PARTIAL` | `upstream_id` exists but is mapped to a clear provider evidence identity |
| P3.5 | Routing decision field | `PARTIAL` | `layer_decision` exists; receipt semantics and candidate snapshot are defined |
| P3.6 | Traffic classification | `PARTIAL` | `traffic_color` exists; receipt meaning is documented or omitted from trust claims |
| P3.7 | Token/cost evidence | `PARTIAL` | detailed counts/cost fields exist; receipt scope/source contract is documented |
| P3.8 | Canonical request hash | `GAP` | canonicalization excludes secrets and produces stable digest for eligible request classes |
| P3.9 | Canonical response hash | `GAP` | streaming/non-streaming behavior is defined and tested |
| P3.10 | Receipt schema + version | `DESIGN REQUIRED` | backward-compatible schema/version policy exists |
| P3.11 | Persist receipt proof fields | `GAP` | receipt hash, signer key id, signature/profile can be queried |
| P3.12 | Sign Trust Receipt | `GAP` | modified receipt fails verification |
| P3.13 | Receipt privacy/security review | `DESIGN REQUIRED` | no bearer secrets, raw provider credentials or unsafe payload leakage |
| P3.14 | Export receipt | `GAP` | customer/auditor can export a stable verification artifact |
| P3.15 | Offline/client receipt verifier | `GAP` | exported artifact can be verified without trusting Console rendering |

### P3 goal verification

Mutating any integrity-covered receipt field after signing must fail verification.

Removing optional upstream evidence must downgrade the chain instead of invalidating unrelated verified links.

## 9. P4 — Provider-specific upstream evidence

Goal: BurnCloud proves only the strongest upstream facts actually supported by each provider path.

Common adapter contract to design:

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
| P4.1 | Generic upstream evidence interface | `DESIGN REQUIRED` | provider adapters return typed evidence + limitations |
| P4.2 | AWS Bedrock evidence adapter | `GAP` | captures only evidence actually available from the AWS path |
| P4.3 | Anthropic evidence adapter | `GAP` | captures only evidence actually available from the Anthropic path |
| P4.4 | Google/Gemini evidence adapter | `GAP` | provider-specific metadata mapped without fabricated parity |
| P4.5 | Azure/OpenAI evidence adapter | `GAP` | provider-specific metadata mapped without fabricated parity |
| P4.6 | Evidence redaction contract | `DESIGN REQUIRED` | secrets/credential material can never enter exported receipts |
| P4.7 | Unsupported-provider state | `GAP` | unknown adapter produces `LIMITED/UNKNOWN`, never fake verification |
| P4.8 | Provider-evidence negative tests | `GAP` | missing request ID/region/metadata downgrades only relevant claims |

### P4 goal verification

For an AWS-labelled channel with missing provider-native evidence, the UI must not claim that AWS execution is cryptographically verified merely because `upstream_id` points to that channel.

## 10. P5 — BurnCloud Client trust UX

Goal: the customer can verify without understanding internal Rust/database implementation.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| P5.1 | Local release verification screen | `GAP` | signature, SHA-256, version/commit and failure reasons visible |
| P5.2 | Connected server verification screen | `GAP` | declared release and runtime-attestation state separated |
| P5.3 | Request list trust column/profile | `GAP` | value has defined profile, not subjective score |
| P5.4 | Request Trust Receipt detail | `GAP` | six-section text prototype can be implemented from real data |
| P5.5 | `Why partial?` explanation | `GAP` | every degraded chain names missing/failed links |
| P5.6 | Export + verify receipt action | `GAP` | verifier can check artifact independently |
| P5.7 | Verification help | `GAP` | UI states what each proof does and does not establish |
| P5.8 | Trust UX accessibility/clarity test | `GAP` | non-expert test user can distinguish declared, signed and attested |

## 11. G1 — API Router product goal

### G1 success statement

Starting from a clean supported environment, an operator can configure upstream supply and produce a successful customer-callable AI API request through BurnCloud.

Current product chain:

```text
Providers → Models → Routes → API access → Playground/real API → Request evidence
```

### G1 TODO

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| G1.1 | Provider configuration | `PARTIAL` | supported provider can be configured with truthful state/error UX |
| G1.2 | Derived model availability | `PARTIAL` | configured vs available vs redundant remain distinct |
| G1.3 | Route/failover configuration story | `PARTIAL` | operator can understand how model traffic can be served |
| G1.4 | Customer/credential ownership | `PARTIAL` | owner identity and bearer-secret lifecycle remain safe |
| G1.5 | Real Playground verification | `PARTIAL` | successful test is persisted as request evidence |
| G1.6 | Clean-environment E2E acceptance | `GAP` | one documented scenario proves setup → successful routed request |

G1 must not be marked done until its successful request can enter the G0 evidence chain.

## 12. G2 — Routing Engine product goal

### G2 success statement

For a routed request, an operator/customer can understand what candidates were eligible, what policy chose the selected upstream, whether fallback occurred, and which evidence proves the actual outcome.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| G2.1 | Provider candidate model | `PARTIAL` | current provider/model/group configuration exists |
| G2.2 | Persist eligible candidate snapshot | `GAP` | receipt can explain candidates at decision time, not only current config |
| G2.3 | Persist selected policy/decision reason | `PARTIAL` | existing `layer_decision` becomes defined receipt evidence |
| G2.4 | Persist failover chain | `PARTIAL` | failover steps are explicit enough for request explanation |
| G2.5 | Health/capacity decision evidence | `DESIGN REQUIRED` | only inputs actually used by router are retained/explained |
| G2.6 | Cost-aware decision evidence | `DESIGN REQUIRED` | if cost participates in routing, exact source/policy is explainable |
| G2.7 | Routing explanation UI | `GAP` | Request detail explains `why this upstream` without reconstructing history from current config |
| G2.8 | Routing invariants tests | `PARTIAL` | existing router tests are extended to receipt/explanation invariants |

## 13. G3 — Business OS product goal

### G3 success statement

An operator can manage a customer lifecycle from account/funding through credentials, usage, upstream cost, customer charge and settlement/margin evidence, while retaining request-level traceability where supported.

| ID | TODO | Status | Acceptance |
| --- | --- | --- | --- |
| G3.1 | Customer account lifecycle | `PARTIAL` | current customer/funding capability mapped to explicit business semantics |
| G3.2 | Credential ownership/lifecycle | `PARTIAL` | account-owned credential management is safe and auditable |
| G3.3 | Usage source contract | `PARTIAL` | account/environment scope is explicit everywhere |
| G3.4 | Billing source contract | `PARTIAL` | billing truth is not reconstructed from bounded Logs samples |
| G3.5 | Upstream cost evidence | `PARTIAL` | current cost fields mapped to exact pricing/source semantics |
| G3.6 | Customer charge model | `DESIGN REQUIRED` | selling price/discount/charge semantics defined independently of upstream cost |
| G3.7 | Margin model | `GAP` | revenue - upstream cost can be explained by period/customer/model/request source |
| G3.8 | Settlement workflow | `DESIGN REQUIRED` | receivable/prepaid/postpaid behavior and authoritative ledger defined |
| G3.9 | Business-to-request drilldown | `GAP` | aggregate metric can reach supporting request evidence when contract permits |

## 14. UI migration backlog after trust semantics freeze

Do not use this as a page-polish checklist. Each item must link to a goal above.

| Order | Surface | Product reason |
| --- | --- | --- |
| 1 | Overview | trust conclusion + highest-value next action |
| 2 | Providers | establish truthful upstream identity/evidence capability |
| 3 | Models | expose model availability without overstating identity proof |
| 4 | Routes | make routing candidates/decisions explainable |
| 5 | Playground | create one real verifiable request |
| 6 | Logs → Requests evolution | make request evidence the primary object |
| 7 | Request Trust Receipt | expose complete per-request chain |
| 8 | Verification | explain release/runtime/evidence profiles |
| 9 | Customers / API Keys | business identity → credential lifecycle |
| 10 | Usage / Billing | business truth with explicit scope |
| 11 | Guardrails / Team / Settings | governance and runtime boundaries |
| 12 | Cross-page visual polish | only after semantics and handoffs converge |

## 15. Verification matrix

Use this table in product reviews. A goal remains open until its final verification row passes.

| Goal | User question | Required evidence | Final verification |
| --- | --- | --- | --- |
| G0 Trust | Can I verify what happened without simply trusting the operator? | signed release proof + truthful runtime state + request receipt + upstream evidence | one real request can be independently evaluated, including limitations |
| G1 Router | Can BurnCloud reliably serve my API request? | configured supply + available model/route + credential + successful request | clean-environment E2E succeeds and persists evidence |
| G2 Routing | Why did this request use this upstream? | candidate snapshot + policy/decision + failover + selected upstream evidence | request detail reconstructs decision from persisted facts, not current config guesses |
| G3 Business | Can I run the customer/token business from authoritative data? | customer + credential + usage + cost + charge/settlement sources | period/customer drilldown reconciles business totals to authoritative sources |

## 16. Per-TODO completion template

Every implementation PR that closes a TODO must answer:

```text
TODO ID:
Goal:
User question:
Claim being added/changed:
Source of truth:
Scope:
Proof/evidence level:
Unknown/loading/error behavior:
Negative test:
Cross-page handoff:
Security/privacy impact:
Automated validation:
Manual/visual validation:
Goal verification result:
```

`Goal verification result` must be one of:

- `PASS` — goal-level acceptance is demonstrably satisfied;
- `PARTIAL` — this TODO moved the goal forward but did not complete it;
- `FAIL` — implementation exists but product acceptance is not met.

## 17. Near-term execution order

The next work should be deliberately narrow:

```text
1. Merge/agree product spine (#420)
2. Agree Product Text Prototype + this TODO
3. Define verification profiles (P0.4)
4. Design signed release manifest + key lifecycle (P1.1/P1.2)
5. Implement official release SHA-256 + signatures (P1)
6. Define Trust Receipt schema/canonical hashes (P3.8-P3.13)
7. Add first upstream evidence adapter (P4)
8. Build client Request Trust Receipt UX (P5)
9. Revalidate Overview and current pages against G0-G3
10. Only then continue broad page migration/pixel polish
```

Runtime attestation (P2.4+) can proceed as a separate architecture track because it is materially harder and platform-dependent. Until it exists, the product must truthfully show `NOT ATTESTED` rather than delaying all other verifiable-evidence work.
