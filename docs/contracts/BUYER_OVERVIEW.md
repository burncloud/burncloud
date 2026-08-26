---
doc_id: contract.buyer-overview
doc_type: runtime-contract
truth: source-derived
status: active
---

# Buyer Overview - Current Contract

## Entry and authentication

`/buyer` redirects to the protected `/buyer/overview` Dioxus route. Both routes run inside `AuthGate`; the existing operator `Overview` at `/` and `/dashboard` is unchanged.

The page sends the current in-memory Console JWT to owner-scoped APIs. `GET /console/api/user/me` resolves the account ID from JWT claims and does not accept a user ID from the client. `GET /console/api/user/recharges`, token listing, and billing summary retain their existing authenticated owner scope.

## Displayed data

The four metrics remain in this order:

1. Today Spend from `GET /api/billing/summary?start=YYYY-MM-DD&end=YYYY-MM-DD`.
2. Balance from `GET /console/api/user/me`.
3. API Availability as `Unknown` because no Buyer-level availability source currently exists.
4. Tokens Today, calculated from the daily billing model summaries.

Dates use UTC. Models in Use is derived only from daily billing model summaries. Recent Activity is derived only from owner-scoped recharge and API-key metadata.

## Truthfulness boundaries

- Missing or failed data is displayed as unavailable or `Unknown`, never as zero or healthy.
- The client does not define a low-balance threshold.
- Model tier and service status are omitted because the current APIs do not provide them.
- The page does not expose an account top-up action because the existing top-up endpoint is administrator-only.

## Source evidence

- `crates/client/src/critical_pages/buyer_overview.rs :: BuyerOverview`
- `crates/client/src/backend.rs :: UserService::current_account`
- `crates/client/src/backend.rs :: billing_summary_for_period`
- `crates/server/src/api/user.rs :: current_account`
- `crates/server/tests/security_invariants.rs :: current_account_is_authenticated_and_owner_scoped`
