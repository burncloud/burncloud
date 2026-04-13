# LLM Review Rules

These rules cover **semantic issues that compiler and CLI tools cannot detect**.
Rules already enforced by clippy/cargo-deny are NOT duplicated here.

## Rules

### R1 — Handler body business logic
Handler functions in `crates/server/src/api/` MUST NOT contain business logic directly.
They should call a service function and return its result.

**Violation example:**
```rust
// BAD: business logic in handler
async fn create_user(Json(input): Json<Value>) -> Json<Value> {
    let hash = bcrypt::hash(&input["password"], 12).unwrap();  // ← logic in handler
    db.insert_user(&input["username"], &hash).await;
    Json(json!({"ok": true}))
}
```

**Correct:**
```rust
// GOOD: handler delegates to service
async fn create_user(Json(input): Json<CreateUserRequest>) -> Result<Json<UserResponse>, AppError> {
    let user = user_service.register(input).await?;
    Ok(Json(user.into()))
}
```

### R2 — No new types in `crates/common`
New domain types (structs/enums representing business entities) MUST be in their domain crate,
not in `crates/common`. The `common` crate is for shared utilities and traits only.

**Violation:** A diff that adds `struct Order`, `struct Payment`, or similar business entities
to any file under `crates/common/src/`.

**Exception:** Infrastructure types (error helpers, trait definitions like `CrudRepository`) are fine.

### R3 — Tests must cover core logic paths
Any new public function added to a service or handler MUST have at least one test.
Tests that only contain `assert!(true)`, empty bodies, or `todo!()` are violations.

**Violation example:**
```rust
#[test]
fn test_create_user() {
    // TODO
}
```

### R4 — New database/service crates must implement CrudRepository
Any new crate under `crates/database/crates/` that introduces a primary domain model
MUST implement `burncloud_common::CrudRepository` for that model.

Any new crate under `crates/service/crates/` that exposes a CRUD service
MUST accept a `CrudRepository` impl, not a concrete database type.

## Output format

Respond with JSON only, no prose:
```json
{
  "passed": true,
  "violations": []
}
```
or
```json
{
  "passed": false,
  "violations": [
    "R1: src/api/user.rs create_user() contains direct bcrypt call — move to service layer",
    "R3: service_group.rs add_member() has no test"
  ]
}
```
