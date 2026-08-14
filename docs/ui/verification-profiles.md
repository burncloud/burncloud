---
doc_id: ui.verification-profiles
doc_type: trust-semantics-standard
truth: target-state
status: draft
---

# BurnCloud Verification Profiles v0.1

This document makes BurnCloud trust labels machine-testable.

It exists to prevent a generic `VERIFIED`, `FULL`, `PARTIAL`, confidence score, or green badge from acquiring different meanings on different pages.

A verification result is always:

```text
claim + profile + evidence + result + limitation reason
```

Never just:

```text
VERIFIED
```

## 1. Core distinction: proof dimensions vs profiles

BurnCloud verifies independent dimensions first. A profile then defines which dimensions are required for a specific claim.

This matters because:

- a valid receipt signature does not prove the server runtime;
- an attested server runtime does not prove the upstream provider;
- an AWS endpoint does not prove a particular model executed there;
- an official client binary does not prove the remote server;
- a persisted request record does not prove integrity after persistence.

The UI must expose those distinctions.

## 2. Result vocabulary

Every verification check returns one of these states:

| Result | Meaning |
| --- | --- |
| `PASS` | required evidence exists and the defined check succeeded |
| `FAIL` | evidence exists but is invalid, mismatched, expired, rejected, or otherwise failed the defined check |
| `UNKNOWN` | the verifier cannot evaluate the check because the proof/source is unavailable, unsupported, unreadable, or not queried |
| `NOT_APPLICABLE` | the check is intentionally outside this request/profile |

`UNKNOWN` and `FAIL` are different.

Examples:

```text
No attestation supplied       → UNKNOWN
Attestation signature invalid → FAIL
Provider adapter unsupported  → UNKNOWN
Provider request ID mismatch  → FAIL
```

No failed cryptographic check may be downgraded to `UNKNOWN` merely to avoid a red state.

## 3. Proof dimensions

### D1 — Official client release provenance

Question:

> Does the local BurnCloud Client match an artifact in an official BurnCloud signed release manifest?

Required checks:

```text
D1.manifest_schema_supported
D1.release_signature_valid
D1.signing_key_accepted
D1.local_artifact_sha256_calculated
D1.local_artifact_sha256_matches_manifest
```

D1 passes only if all required checks pass.

D1 does not prove the remote server.

### D2 — Official server release claim

Question:

> Does the remote server's claimed release identity correspond to an official signed BurnCloud release artifact?

Required checks:

```text
D2.server_claim_present
D2.release_manifest_signature_valid
D2.signing_key_accepted
D2.claimed_version_matches_manifest
D2.claimed_build_identity_matches_manifest
D2.claimed_server_artifact_exists_in_manifest
```

D2 means the **claim references a valid official release**.

D2 does not prove the remote process is executing the declared artifact.

### D3 — Remote runtime attestation

Question:

> Does accepted platform/hardware-backed attestation prove a fresh runtime measurement that satisfies BurnCloud's attestation policy?

Required checks:

```text
D3.attestation_supported
D3.attestation_signature_chain_valid
D3.challenge_nonce_matches
D3.attestation_fresh
D3.measurement_accepted
D3.release_measurement_binding_valid
D3.receipt_signing_identity_binding_valid
```

The final binding is critical: an attested runtime must bind the request receipt signer (or a session/key derivation chain) to the attested workload. Otherwise an attacker could present a valid attestation from one process and sign receipts from another.

D3 design must define the concrete platform before implementation.

### D4 — Receipt integrity

Question:

> Is this Request Trust Receipt structurally supported and cryptographically intact?

Required checks:

```text
D4.schema_supported
D4.canonicalization_supported
D4.canonical_receipt_hash_matches
D4.signature_present
D4.signer_key_id_present
D4.signer_key_resolved
D4.receipt_signature_valid
```

D4 proves integrity/authenticity relative to the resolved signer identity.

Without D3, D4 does not prove the signer is running an official BurnCloud runtime.

### D5 — Local request/response binding

Question:

> Does the receipt describe the same request and response observed by the verifying client?

Required checks for request classes that support canonical hashing:

```text
D5.request_hash_present
D5.local_request_hash_calculated
D5.local_request_hash_matches_receipt
D5.response_hash_present
D5.local_response_hash_calculated
D5.local_response_hash_matches_receipt
```

Streaming requires a separately specified canonical transcript or hash-chain algorithm. Until that specification exists, streaming request/response binding must remain unsupported/unknown rather than reusing a non-streaming hash incorrectly.

D5 is applicable only when the verifier possesses the corresponding local request/response material.

### D6 — Persisted routing decision evidence

Question:

> Does the receipt contain enough persisted facts to explain the route decision at request time without reconstructing history from current configuration?

Minimum target checks:

```text
D6.requested_model_present
D6.route_group_or_policy_scope_present
D6.eligible_candidate_snapshot_present
D6.selected_upstream_present
D6.decision_reason_present
D6.failover_chain_present_or_explicit_none
```

Current `layer_decision` is a useful primitive but is not alone sufficient for D6.

### D7 — Upstream evidence profile

Question:

> Does the selected provider adapter supply the evidence required by the declared provider verification profile?

Common checks:

```text
D7.provider_family_present
D7.endpoint_identity_present
D7.resolved_provider_model_present
D7.provider_evidence_profile_known
D7.profile_required_fields_present
D7.profile_consistency_checks_pass
```

Additional checks are provider-specific, for example region or provider-native request metadata where available.

D7 must not pretend all providers expose equivalent proof.

### D8 — Financial evidence consistency

Question:

> Are receipt-level cost/usage fields internally consistent with the authoritative pricing/billing source contract used for this request?

Target checks may include:

```text
D8.token_counts_present
D8.cost_status_ok_or_explained
D8.pricing_region_present_when_required
D8.upstream_cost_source_known
D8.customer_charge_source_known_when_applicable
```

D8 is not required for basic request identity verification, but is required by business verification profiles that claim cost/charge traceability.

### D9 — Privacy-safe export

Question:

> Can this proof artifact be shared with the intended verifier without exposing secrets?

Required safety checks:

```text
D9.no_bearer_secret
D9.no_raw_provider_credential
D9.no_sensitive_authorization_header
D9.identities_redacted_or_scoped_by_policy
D9.payload_disclosure_matches_export_policy
```

D9 is a mandatory export gate. A receipt that fails D9 must not be exportable even if other cryptographic checks pass.

## 4. Request evidence maturity states

These are **maturity descriptions**, not verification profiles.

### `RECORDED`

A request has persisted operational fields such as request identity, selected upstream and result.

This is not a cryptographic trust claim.

### `SIGNED RECEIPT`

D4 passes.

Allowed wording:

```text
Receipt integrity: SIGNATURE VERIFIED
Server runtime: NOT ATTESTED / UNKNOWN
```

Do not render `CHAIN VERIFIED` from D4 alone.

### `UPSTREAM EVIDENCED`

D4 + D6 + D7 pass for a declared provider evidence profile.

Allowed wording:

```text
Receipt integrity: SIGNATURE VERIFIED
Routing evidence: PASS
Upstream evidence (AWS profile v1): PASS
Runtime attestation: UNKNOWN
```

This is still not a runtime-bound chain.

### `RUNTIME BOUND`

D3 + D4 pass and D3 proves the receipt signing identity is bound to the accepted attested runtime.

This proves a stronger server-to-receipt link but still does not automatically satisfy D7 upstream evidence.

## 5. Named verification profiles

A profile is a versioned set of required dimensions.

The profile name must be shown next to `CHAIN VERIFIED`.

### VP1 — `RECEIPT_INTEGRITY_V1`

Purpose:

> Verify that the Trust Receipt is intact and signed by a resolved receipt signer.

Required:

```text
D4 = PASS
```

Optional/not required:

```text
D1, D2, D3, D5, D6, D7, D8
```

Result wording:

```text
Receipt signature: SIGNATURE VERIFIED
Profile: RECEIPT_INTEGRITY_V1 PASS
```

Never:

```text
Request chain VERIFIED
```

### VP2 — `ROUTE_EVIDENCE_V1`

Purpose:

> Verify receipt integrity plus persisted routing explanation.

Required:

```text
D4 = PASS
D6 = PASS
```

Result wording:

```text
Profile: ROUTE_EVIDENCE_V1 PASS
Runtime: NOT INCLUDED
Upstream identity: NOT INCLUDED
```

### VP3 — `UPSTREAM_EVIDENCE_V1`

Purpose:

> Verify an intact receipt, persisted route decision and provider-specific upstream evidence.

Required:

```text
D4 = PASS
D6 = PASS
D7 = PASS
```

Result wording:

```text
Profile: UPSTREAM_EVIDENCE_V1 PASS
Provider profile: <provider-profile-name/version>
Runtime attestation: NOT INCLUDED
```

This profile may support a customer claim such as:

> BurnCloud retained evidence consistent with the declared AWS Bedrock upstream profile for this request.

It must not claim hardware-attested BurnCloud runtime execution.

### VP4 — `RUNTIME_BOUND_REQUEST_V1`

Purpose:

> Verify that an intact receipt was signed by an identity bound to an accepted fresh BurnCloud runtime attestation.

Required:

```text
D2 = PASS
D3 = PASS
D4 = PASS
D6 = PASS
```

Result wording:

```text
CHAIN VERIFIED — RUNTIME_BOUND_REQUEST_V1
Upstream evidence: separate dimension
```

The profile does not imply D7 unless explicitly composed with an upstream profile.

### VP5 — `CLIENT_TO_UPSTREAM_V1`

Purpose:

> Highest target customer-verifier profile: verify local official client provenance, bind the locally observed request/response to a runtime-bound receipt, and verify provider-specific upstream evidence.

Required:

```text
D1 = PASS
D2 = PASS
D3 = PASS
D4 = PASS
D5 = PASS
D6 = PASS
D7 = PASS
```

D9 must pass before export.

Result wording:

```text
CHAIN VERIFIED — CLIENT_TO_UPSTREAM_V1
```

This label is permitted only when every required dimension passes.

Financial traceability is not included unless a future business profile also requires D8.

### VP6 — `BUSINESS_TRACE_V1`

Purpose:

> Verify request-level usage/cost/charge traceability for Business OS workflows.

Target required dimensions:

```text
D4 = PASS
D6 = PASS
D8 = PASS
```

If the business claim includes upstream identity, compose with VP3 requirements.

The exact authoritative customer-charge/ledger contract must be designed before VP6 can become implementable.

## 6. Overall UI chain state

The UI may show a simple chain summary for orientation, but it must derive from profile results.

### `UNKNOWN`

No meaningful profile could be evaluated because required proof is unavailable or unsupported.

### `FAILED`

At least one explicitly evaluated security/integrity check failed for the selected profile.

A failure must be prominent and must not be hidden behind a lower profile that happened to pass unless the user explicitly switches profiles.

### `PARTIAL`

One or more lower profiles pass, but the highest expected/requested profile cannot pass because required evidence is `UNKNOWN`, `NOT_APPLICABLE`, or not yet supported.

The UI must list missing dimensions.

Example:

```text
PARTIAL
✓ Receipt integrity
✓ Routing evidence
✓ AWS upstream evidence
? Runtime attestation unavailable
```

### `CHAIN VERIFIED — <PROFILE>`

Every required dimension for the named profile passes.

The profile name is mandatory.

Do not use bare `FULL` as a technical verification state.

## 7. Downgrade and failure rules

1. Missing evidence downgrades only profiles that require that evidence.
2. Invalid evidence fails every profile that requires the failed dimension.
3. A passed lower profile remains factually passed, but cannot mask a failed higher-profile check.
4. Unsupported providers may pass receipt integrity/routing profiles while upstream-evidence profiles remain `UNKNOWN`.
5. No runtime attestation means VP4/VP5 cannot pass.
6. No local request/response material means D5 is `NOT_APPLICABLE` for server-side/auditor-only verification; VP5 cannot be claimed in that context.
7. Receipt signer identity must never be treated as official runtime identity unless D3 binding passes.
8. Clock/freshness uncertainty must fail or mark unknown according to the concrete proof specification; never silently ignore expiry.

## 8. Example results

### Example A — current-style log evidence only

```text
request_id             present
upstream_id            present
layer_decision         present
receipt signature      unavailable
provider evidence      limited
runtime attestation    unavailable

Result:
RECORDED
No verification profile passed.
```

### Example B — signed receipt + AWS evidence, no runtime attestation

```text
D4 Receipt integrity   PASS
D6 Routing evidence    PASS
D7 AWS evidence v1     PASS
D3 Runtime             UNKNOWN

Profiles:
VP1 PASS
VP2 PASS
VP3 PASS
VP4 UNKNOWN
VP5 UNKNOWN

UI:
PARTIAL
Strongest passed profile: UPSTREAM_EVIDENCE_V1
Missing for runtime-bound verification: runtime attestation
```

### Example C — attested server but unsupported provider evidence

```text
D2 Server release      PASS
D3 Runtime             PASS
D4 Receipt integrity   PASS
D6 Routing evidence    PASS
D7 Provider evidence   UNKNOWN

Profiles:
VP1 PASS
VP2 PASS
VP4 PASS
VP3 UNKNOWN
VP5 UNKNOWN

UI:
CHAIN VERIFIED — RUNTIME_BOUND_REQUEST_V1
Upstream verification: UNKNOWN / unsupported profile
```

### Example D — receipt signature mismatch

```text
D4 Receipt integrity   FAIL

Result:
FAILED
Reason: receipt signature mismatch
```

The UI must not fall back to `RECORDED` and present a reassuring green state after a signature failure.

## 9. Machine-readable target

Future implementation should expose a structured verification result rather than deriving labels from prose in the UI.

Conceptual shape:

```text
verification_result
  schema_version
  requested_profile
  overall_result
  strongest_passed_profile
  dimensions[]
    id
    result
    evidence_type
    reason_code
    detail_safe
  limitations[]
  evaluated_at
```

UI copy should map from stable `reason_code` values, not parse free-form error strings.

## 10. Acceptance tests for P0.4

P0.4 is `DEFINED` when product/engineering agree that:

- every generic verification label maps to a named profile;
- `PASS`, `FAIL`, `UNKNOWN`, `NOT_APPLICABLE` are distinct;
- D4 receipt integrity is explicitly insufficient for runtime trust;
- D3 binds receipt signing identity to the attested workload;
- provider evidence is a separate dimension from runtime proof;
- a bare `FULL`/`VERIFIED` label is forbidden;
- every partial result can list exactly which dimensions are missing;
- negative evidence causes failure, not silent downgrade;
- the highest target customer profile is `CLIENT_TO_UPSTREAM_V1` (or an explicitly approved successor);
- future code can represent the result structurally without relying on UI wording.

After agreement, implementation work should use these profiles as acceptance truth until a versioned successor is approved.
