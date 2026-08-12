---
doc_id: agent.playbook.provider-integration
doc_type: agent-playbook
truth: normative
status: active
---

# Provider Integration Playbook

Use this when adding or materially changing an upstream provider/adaptor execution path.

Provider work is high risk because it can change request semantics, streaming, usage/billing, retries, errors, and provenance at once.

## First: find the existing provider architecture

Do not assume every provider follows the same static path.

Identify from source:

- registration/selection mechanism;
- request conversion or passthrough boundary;
- authentication/credential source;
- endpoint construction;
- streaming and non-streaming paths;
- response/error conversion;
- usage extraction;
- retry/failover ownership;
- billing/trace integration.

Label runtime-selected edges `DYNAMIC`.

## Integration checklist

Inspect and define behavior for:

- request conversion;
- model mapping;
- endpoint/versioning;
- auth headers/signing;
- headers/query parameters;
- non-streaming response;
- streaming response/chunk parsing;
- tool/function calls if supported;
- structured output if supported;
- finish/stop reason mapping;
- upstream errors/status codes;
- timeout/cancellation;
- retry/failover semantics;
- usage/token parsing;
- billing/cost settlement;
- trace/provenance fields;
- secrets/log redaction.

Not every provider implements every capability. Unsupported behavior should be explicit rather than guessed.

## Verification minimum

A meaningful provider change should normally cover more than a happy path. Depending on capabilities, inspect/run:

1. non-streaming success;
2. streaming success;
3. upstream error mapping;
4. timeout/cancellation or retry path;
5. usage parsing;
6. billing/settlement implications;
7. provider-specific regression tests;
8. relevant end-to-end relay tests.

Use `../TEST_MATRIX.md` to find current suites instead of inventing test paths.
