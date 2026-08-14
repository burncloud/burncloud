---
doc_id: ui.product-text-prototype
doc_type: product-prototype
truth: target-state-with-source-audit
status: draft
audited_against: c314bff9646f9113c9a58a818552fc80c77543a6
---

# BurnCloud Product Text Prototype v0.1

This document defines the target product experience before visual design or page implementation.

It is deliberately text-first. The purpose is to decide **what BurnCloud must prove, what each screen must communicate, and what the user should do next** before spending time on CSS, component polish, or page-specific implementation.

`product-flow.md` remains the current Console responsibility contract. This prototype is the target-state product model that future implementation PRs must validate against.

## 1. North star

BurnCloud's primary product priority is **trust through independent verification**.

The product must not ask a customer to trust a BurnCloud operator, a screenshot, a version string, or a marketing claim. It should expose enough evidence for the customer to independently determine what can and cannot be verified.

Primary product statement:

> **BurnCloud is a verifiable AI gateway. Every AI request should have an explainable and verifiable chain of custody.**

Customer-facing promise:

> **Do not trust our claim. Verify the software, the request path, and the upstream evidence yourself.**

The other product pillars exist under this north star:

```text
                         TRUST
                  Verifiable AI Gateway
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               ▼               ▼
     API ROUTER      ROUTING ENGINE    BUSINESS OS
     can serve       stable/fast/      can operate
     AI traffic      cost-aware        the business
          │               │               │
          └───────────────┴───────────────┘
                          │
                must remain explainable
                  and evidence-backed
```

Priority order when goals conflict:

1. truthful / verifiable claims;
2. stable and understandable routing;
3. complete business operations;
4. convenience and visual polish.

A visually better screen must never win over a more truthful screen.

## 2. Primary user and proof scenario

The highest-priority trust scenario is the **customer verifying the service sold to them**.

Example:

1. a customer buys access that is represented as AWS Claude;
2. the customer uses the BurnCloud Client;
3. the client verifies its own official release provenance;
4. the client connects to a BurnCloud Server and evaluates what can be proven about that server runtime;
5. the customer's request receives a stable request identity;
6. BurnCloud records the route decision and actual upstream evidence;
7. the client presents a Request Trust Receipt;
8. the customer can distinguish facts that are cryptographically verified, operationally evidenced, merely declared, or unknown.

Secondary users:

- **Operator** — configures providers, routing, customers, credentials, guardrails and maintenance.
- **Customer verifier** — checks the service identity and individual request evidence.
- **Auditor / partner** — evaluates exported evidence without needing Console mutation privileges.
- **Developer** — maps product claims to source code, tests and provider adapters.

The customer-verifier path has priority over operator convenience when trust semantics conflict.

## 3. Trust chain

The target chain is:

```text
Official BurnCloud source
        │
        ▼
Signed release manifest
        │
        ├── version
        ├── git commit
        ├── source/build identity
        ├── client SHA-256
        ├── server SHA-256
        └── BurnCloud release signature
        │
        ▼
BurnCloud Client verifies local package
        │
        ▼
Client evaluates BurnCloud Server identity
        │
        ▼
Request enters BurnCloud
        │
        ├── request identity
        ├── customer / credential attribution
        ├── requested model
        ├── eligible route evidence
        ├── selected upstream
        ├── fallback / routing decision
        └── canonical request hash
        │
        ▼
Actual upstream call
        │
        ├── provider identity evidence
        ├── endpoint / region evidence
        ├── resolved provider model
        ├── provider request metadata where available
        └── TLS / transport evidence where meaningful
        │
        ▼
Response returns
        │
        ├── HTTP result
        ├── token usage
        ├── latency
        ├── cost evidence
        ├── canonical response hash
        └── receipt signature
        │
        ▼
Request Trust Receipt
```

A break in the chain must remain visible. BurnCloud must not convert a partial chain into a green end-to-end `VERIFIED` claim.

## 4. Claim levels — what BurnCloud is allowed to say

Trust vocabulary must be stricter than normal operational status vocabulary.

| Trust state | Meaning | Allowed example | Forbidden interpretation |
| --- | --- | --- | --- |
| `UNKNOWN` | source was not read or proof is missing | `Runtime identity: UNKNOWN` | healthy / false / zero |
| `DECLARED` | a component reported an identity without independent proof | `Server declares v1.8.2` | official runtime verified |
| `HASH MATCH` | bytes observed locally match an expected SHA-256 | `Client binary: HASH MATCH` | signer identity verified |
| `SIGNATURE VERIFIED` | a signed manifest/receipt validates against an expected public key | `Release manifest: SIGNATURE VERIFIED` | remote runtime necessarily running those bytes |
| `EVIDENCED` | an operational fact is supported by persisted/request/provider evidence | `AWS endpoint: EVIDENCED` | cryptographically attested provider execution |
| `RUNTIME ATTESTED` | remote measurement is verified against an accepted attestation policy | `Server runtime: RUNTIME ATTESTED` | every upstream response is independently attested |
| `CHAIN VERIFIED` | all required proof links for a defined verification profile pass | `Request chain: CHAIN VERIFIED` | proof beyond the declared profile |

### Critical wording rule

`VERIFIED` is not a decoration. Every `VERIFIED` label must name the verification profile or the exact proof that passed.

Bad:

```text
Server VERIFIED
```

Good:

```text
Release signature     SIGNATURE VERIFIED
Client binary hash    HASH MATCH
Server runtime        UNKNOWN
Request receipt       SIGNATURE VERIFIED
AWS endpoint          EVIDENCED
```

## 5. MD5 policy

MD5 may be displayed only as a compatibility or legacy file fingerprint if required by an external workflow.

It must not be the security basis for official BurnCloud release trust.

The target release identity uses at least:

```text
Version
Git commit
SHA-256 digest
Signed release manifest
Signer identity / public-key fingerprint
```

## 6. Release proof model

Target artifact:

```text
release-manifest.json
```

Conceptual fields:

```text
schema_version
release_version
git_commit
source_tree_identity
build_profile
build_timestamp
client_artifacts[]
  platform
  filename
  sha256
server_artifacts[]
  platform
  filename
  sha256
signing_key_id
signature
```

The exact canonicalization/signature format must be designed before implementation. The product prototype intentionally does not select a cryptographic library or serialization scheme.

Client behavior:

```text
Local BurnCloud Client
        │
        ├── read local binary/package identity
        ├── obtain official signed manifest
        ├── verify BurnCloud release signature
        ├── calculate local SHA-256
        └── compare artifact digest
        │
        ▼
SIGNATURE VERIFIED + HASH MATCH
```

A successful local release check proves the local artifact matches a signed BurnCloud release artifact. It does **not** by itself prove what a remote server is currently executing.

## 7. Server runtime proof model

The product must explicitly separate three generations of server identity.

### Stage R0 — declared identity

Server returns version/build information.

UI wording:

```text
Server release       v1.8.2
Runtime proof        DECLARED ONLY
```

This is useful diagnostics but not independent runtime proof.

### Stage R1 — signed release identity

The client can validate a signed release manifest associated with the server's claimed build.

UI wording:

```text
Claimed release      v1.8.2
Release signature    SIGNATURE VERIFIED
Remote runtime       NOT ATTESTED
```

This proves the release exists and is officially signed. It still does not prove the remote process is executing those exact bytes.

### Stage R2 — remote runtime attestation

A supported deployment can return a hardware/platform-backed measurement that the client validates against an accepted measurement and policy.

Possible future mechanisms include TEE/TPM-backed remote attestation. Exact infrastructure is an architecture decision, not assumed by this prototype.

UI wording:

```text
Server release       v1.8.2
Release signature    SIGNATURE VERIFIED
Runtime measurement  MATCH
Attestation          RUNTIME ATTESTED
```

Only this stage may support a strong remote-runtime verification profile.

## 8. Request Trust Receipt

The Request Trust Receipt is the central target product object.

It is not a prettier log row. It is the evidence package for one routed AI request.

Target conceptual schema:

```text
Receipt Identity
  receipt_schema
  receipt_id
  request_id
  created_at

Software Identity
  client_release
  client_release_proof
  server_release_claim
  server_runtime_proof

Request Identity
  customer_id / privacy-safe identity
  credential_id / management-safe identity
  requested_model
  request_hash

Routing Decision
  route_group
  eligible_upstreams
  selected_upstream
  routing_policy
  layer_decision
  fallback_chain

Upstream Evidence
  provider_family
  provider_account_identity (privacy-safe)
  provider_region
  endpoint_identity
  resolved_provider_model
  upstream_request_id / provider metadata when available
  provider_evidence_profile

Response Evidence
  status_code
  latency_ms
  prompt_tokens
  completion_tokens
  cache_tokens
  reasoning_tokens
  response_hash

Financial Evidence
  pricing_region
  cost_status
  upstream_cost
  customer_charge when available

Receipt Proof
  canonical_receipt_hash
  signer_key_id
  receipt_signature
  verification_profile
```

Existing fields such as `request_id`, `upstream_id`, `layer_decision`, `traffic_color`, `cost_status`, token counts and cost are useful foundations, but they are not yet equivalent to a complete Trust Receipt.

### Receipt signature limitation

A signed receipt binds fields to a signing key. That alone does not prove the signer ran an official BurnCloud runtime.

The UI therefore must keep these separate:

```text
Receipt signature       SIGNATURE VERIFIED
Signer identity         KNOWN / UNKNOWN
Server runtime          DECLARED / RUNTIME ATTESTED
Upstream evidence       EVIDENCED / UNKNOWN
Overall verification    profile-dependent
```

## 9. Upstream proof model

Different providers expose different evidence. BurnCloud must use provider-specific evidence adapters instead of inventing one universal proof claim.

Evidence categories may include:

- configured provider family;
- resolved endpoint host;
- TLS certificate/connection result where available and meaningful;
- cloud region;
- provider-native request ID or response metadata;
- resolved provider model identifier;
- account/credential alias represented in a privacy-safe way;
- provider-specific signed or attestable metadata if a provider supports it.

Target UI rule:

> Show the strongest evidence actually available for that provider, and show `UNKNOWN` for missing proof.

Never infer:

- `upstream_id = AWS` ⇒ AWS execution cryptographically verified;
- an HTTPS hostname ⇒ model authenticity proven;
- a successful Claude-like response ⇒ Anthropic model identity proven;
- a configured provider ⇒ this request actually used that provider.

## 10. Target information architecture

This is the target product model, not an instruction to rename every route immediately.

```text
Overview

TRAFFIC
  Providers
  Models
  Routes
  Playground

TRUST
  Requests
  Verification

BUSINESS
  Customers
  API Keys
  Usage
  Billing

CONTROL
  Guardrails
  Team
  Settings
```

### Migration note

Current `Logs` may evolve into `Requests` once the request evidence object is strong enough. Do not rename the route merely for branding before the underlying product semantics change.

A separate `Evidence` page is not required if Request detail + Verification can make the chain understandable. Avoid creating navigation merely to mirror database concepts.

## 11. Text prototype — BurnCloud Client startup

```text
┌──────────────────────────────────────────────────────────────┐
│ BurnCloud Client                                             │
│ Verify before you trust                                      │
├──────────────────────────────────────────────────────────────┤
│ LOCAL SOFTWARE                                               │
│                                                              │
│ Version                 v1.8.2                               │
│ Git commit              c314bff                              │
│ Release signature       SIGNATURE VERIFIED                   │
│ Client SHA-256          HASH MATCH                           │
│                                                              │
│ [View signed manifest]                                       │
├──────────────────────────────────────────────────────────────┤
│ CONNECTED SERVER                                             │
│                                                              │
│ Endpoint                https://gateway.example.com           │
│ Claimed release         v1.8.2                               │
│ Release signature       SIGNATURE VERIFIED                   │
│ Runtime proof           NOT ATTESTED                         │
│                                                              │
│ This server matches an official release claim, but the       │
│ remote process has not supplied hardware-backed attestation. │
│                                                              │
│ [Open Verification]                                          │
└──────────────────────────────────────────────────────────────┘
```

The important behavior is the truthful limitation message. The UI must not turn `NOT ATTESTED` into a green generic `VERIFIED` banner.

## 12. Text prototype — Overview

```text
┌────────────────────────────────────────────────────────────────────┐
│ OVERVIEW                                                           │
│ Trust first. Operate from evidence.                                │
├────────────────────────────────────────────────────────────────────┤
│ TRUST STATUS                                                       │
│                                                                    │
│ Client release        SIGNATURE VERIFIED / HASH MATCH              │
│ Server runtime        NOT ATTESTED                                 │
│ Request evidence      98.7% receipt coverage                       │
│ Upstream evidence     7 evidenced / 1 limited / 1 unknown          │
│                                                                    │
│ Primary conclusion:                                               │
│ Request traffic is operating, but remote runtime identity is       │
│ not yet independently attested.                                   │
│                                                                    │
│ [Open Verification]                                                │
├────────────────────────────────────────────────────────────────────┤
│ NEEDS ATTENTION                                                    │
│                                                                    │
│ • 12 requests have incomplete upstream evidence   → Open Requests │
│ • 1 provider has no active supply                 → Providers     │
│                                                                    │
├────────────────────────────────────────────────────────────────────┤
│ OPERATING FLOW                                                     │
│                                                                    │
│ Supply     AVAILABLE       8 providers / 21 models / 4 groups     │
│ Access     AVAILABLE       55 active credentials                  │
│ Traffic    OBSERVED        1,240 recent requests                  │
│ Business   AVAILABLE       billing source loaded                  │
│                                                                    │
├────────────────────────────────────────────────────────────────────┤
│ LATEST REQUEST                                                     │
│                                                                    │
│ req_01J...                                                         │
│ claude-sonnet → AWS Bedrock → HTTP 200                            │
│ Trust receipt: PARTIAL CHAIN                                      │
│                                                                    │
│ [Open Request]                                                     │
└────────────────────────────────────────────────────────────────────┘
```

Overview owns the conclusion and handoff only. It must not become the detailed release verifier, provider inspector, request receipt, or billing ledger.

## 13. Text prototype — Requests

Target evolution of today's operational Logs surface:

```text
┌────────────────────────────────────────────────────────────────────┐
│ REQUESTS                                                           │
│ Every routed request and the evidence BurnCloud retained.          │
├────────────────────────────────────────────────────────────────────┤
│ Search request ID...                 [Trust: Any ▼] [Provider ▼]   │
├────────────────────────────────────────────────────────────────────┤
│ TIME      REQUEST       MODEL         UPSTREAM       RESULT  TRUST  │
│ 21:42     req_01J...    sonnet        AWS Bedrock    200     FULL   │
│ 21:41     req_01K...    sonnet        Anthropic     200     PARTIAL│
│ 21:40     req_01L...    gemini        UNKNOWN       502     LIMITED│
│                                                                    │
│ Selecting a row opens its Request Trust Receipt.                   │
└────────────────────────────────────────────────────────────────────┘
```

The Trust column represents a defined verification profile, not a subjective confidence score.

## 14. Text prototype — Request Trust Receipt

```text
┌────────────────────────────────────────────────────────────────────┐
│ REQUEST TRUST RECEIPT                                              │
│ req_01J...                                        PARTIAL CHAIN    │
├────────────────────────────────────────────────────────────────────┤
│ 1. SOFTWARE                                                       │
│ Client release          v1.8.2  SIGNATURE VERIFIED / HASH MATCH   │
│ Server release          v1.8.2  SIGNATURE VERIFIED                │
│ Server runtime                  NOT ATTESTED                       │
├────────────────────────────────────────────────────────────────────┤
│ 2. REQUEST                                                        │
│ Customer                customer_***                              │
│ Credential              key_mgmt_***                              │
│ Requested model         claude-sonnet                             │
│ Request hash            sha256:...                                │
├────────────────────────────────────────────────────────────────────┤
│ 3. ROUTING DECISION                                               │
│ Route group             premium-claude                            │
│ Eligible upstreams      AWS / Anthropic                           │
│ Selected                AWS Bedrock                               │
│ Decision                primary                                  │
│ Failover                none                                     │
├────────────────────────────────────────────────────────────────────┤
│ 4. UPSTREAM EVIDENCE                                              │
│ Provider                AWS Bedrock                               │
│ Region                  us-east-1                                 │
│ Endpoint                bedrock-runtime...                        │
│ Provider model          anthropic.claude-...                      │
│ Provider request ID     ...                                       │
│ Evidence profile        EVIDENCED                                 │
├────────────────────────────────────────────────────────────────────┤
│ 5. RESPONSE                                                       │
│ HTTP                    200                                       │
│ Latency                 1,284 ms                                  │
│ Prompt / completion     12,482 / 2,194                            │
│ Response hash           sha256:...                                │
├────────────────────────────────────────────────────────────────────┤
│ 6. RECEIPT PROOF                                                  │
│ Receipt hash            sha256:...                                │
│ Receipt signature       SIGNATURE VERIFIED                        │
│ Runtime attestation     NOT AVAILABLE                             │
│                                                                    │
│ Why PARTIAL CHAIN?                                                 │
│ The request and upstream evidence are signed and retained, but     │
│ this server did not provide an accepted remote-runtime attestation.│
│                                                                    │
│ [Export Receipt] [Verify Receipt]                                 │
└────────────────────────────────────────────────────────────────────┘
```

The explanation of *why* a chain is partial is mandatory. A score without reasons is not useful trust UX.

## 15. Text prototype — Verification

```text
┌────────────────────────────────────────────────────────────────────┐
│ VERIFICATION                                                       │
│ Independently inspect BurnCloud software and proof capabilities.   │
├────────────────────────────────────────────────────────────────────┤
│ RELEASE                                                           │
│ Version                 v1.8.2                                    │
│ Commit                  c314bff                                   │
│ Manifest                SIGNATURE VERIFIED                         │
│ Signer                  BurnCloud Release Key ...                  │
│ Client SHA-256          HASH MATCH                                │
│ Server artifact SHA-256 manifest-known                            │
│                                                                    │
│ [View Manifest] [Copy Digests]                                    │
├────────────────────────────────────────────────────────────────────┤
│ RUNTIME                                                           │
│ Server claim            v1.8.2                                    │
│ Runtime attestation     NOT AVAILABLE                             │
│                                                                    │
│ What this proves: official release metadata can be verified.       │
│ What this does not prove: exact remote process bytes at this time. │
├────────────────────────────────────────────────────────────────────┤
│ REQUEST EVIDENCE PROFILES                                         │
│ Signed receipt          AVAILABLE                                 │
│ AWS evidence adapter    AVAILABLE                                 │
│ Anthropic adapter       LIMITED                                   │
│ Runtime-bound receipt   NOT AVAILABLE                             │
└────────────────────────────────────────────────────────────────────┘
```

Verification is an explanation surface. It must always expose both **what is proven** and **what is not proven**.

## 16. Text prototype — Traffic

Traffic configuration remains operationally simple:

```text
Providers → Models → Routes → Playground
```

But each page must help produce evidence for the trust chain.

### Providers

Must answer:

- what provider is configured;
- what provider family BurnCloud believes it is;
- which endpoint/region/account alias will be used;
- what upstream evidence profile the adapter can collect;
- current operational availability.

### Models

Must answer:

- which model IDs are derived from providers;
- which are currently available;
- which have multiple active upstreams;
- which upstream identity profiles can support them.

### Routes

Must answer:

- what candidates are eligible;
- priority/weight/policy;
- failover behavior;
- what routing facts will be retained in the request receipt.

### Playground

Must be the easiest way to create one real request and immediately open its Request Trust Receipt.

The success path should become:

```text
Send test request
       ↓
Receive response
       ↓
Persist request evidence
       ↓
Open Trust Receipt
       ↓
Verify what happened
```

## 17. Text prototype — Business OS

Business capability remains important, but it must inherit trust semantics instead of competing with them.

Target business chain:

```text
Customer
   ↓
Wallet / commercial terms
   ↓
API Key
   ↓
Request
   ↓
Usage
   ↓
Upstream cost
   ↓
Customer charge
   ↓
Margin / settlement
```

Future business pages must be able to trace aggregate numbers back to request evidence when the source contracts permit it.

Do not reconstruct billing truth from an arbitrary bounded log sample.

## 18. Product completion loop

No feature is complete because its UI exists.

Every product task follows:

```text
GOAL
  ↓
TEXT PROTOTYPE
  ↓
TODO WITH ACCEPTANCE CRITERIA
  ↓
IMPLEMENTATION
  ↓
SOURCE-OF-TRUTH CHECK
  ↓
STATE / ERROR CHECK
  ↓
CROSS-PAGE HANDOFF CHECK
  ↓
EXECUTABLE CONTRACT / TEST
  ↓
VISUAL REVIEW
  ↓
GOAL VERIFICATION
```

If goal verification fails, reopen the TODO even when implementation and CI are green.

## 19. Definition of done for trust features

A trust-related feature is complete only when:

1. the exact claim is defined;
2. the source of proof is named;
3. missing proof has an explicit `UNKNOWN`/`NOT AVAILABLE` state;
4. the UI distinguishes declaration, evidence, signature verification and runtime attestation;
5. the proof can be independently checked by the intended verifier;
6. the result explains why it passed, failed or remained partial;
7. negative tests prove BurnCloud does not overclaim when evidence is absent or invalid;
8. exported evidence does not leak credentials or sensitive raw secrets;
9. the product TODO links to implementation and verification evidence;
10. the north-star question is answered: **does this make the customer's request chain more independently verifiable?**

## 20. Immediate product decision

Until this prototype and the linked TODO are reviewed, new page-polish work should not introduce additional navigation concepts, trust scores, generic `VERIFIED` labels, or new ownership boundaries.

Existing draft implementation PRs are candidates to be re-validated against this prototype rather than treated as acceptance truth merely because they compile.
