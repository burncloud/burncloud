# BurnCloud Service Models

Supply-side model resolution and HuggingFace discovery helpers.

## What this crate owns

- model manifest / variant resolution;
- local-model resolver contracts and implementations;
- HuggingFace model/file discovery;
- model download URL and local data-directory helpers.

## What this crate does not own

It does **not** expose database CRUD for HuggingFace repository metadata. The previous `ModelService`
CRUD facade called seven no-op methods in `burncloud-database-model`; #621 removed that false
capability instead of preserving an API that never persisted anything.

Runtime model capability truth is persisted by `burncloud-database-model::ModelCapabilityModel`.
Pricing remains Commerce-owned.

## HuggingFace example

```rust
let models = ModelService::fetch_from_huggingface().await?;
```
