# model_capabilities database design

This crate owns the runtime persistence boundary for Supply model capability truth.

## Existing physical table

The historical `model_capabilities` table contains:

```text
id
model
context_window
max_output_tokens
supports_vision
supports_function_calling
input_price
output_price
synced_at
```

The table is intentionally **not** rewritten by #621. The two price columns are legacy compatibility
projections. Canonical pricing lives in Commerce (`billing_prices` and its contracts); callers must
not treat `model_capabilities.input_price/output_price` as a second pricing source of truth.

## Rust mapping

- `ModelCapability` maps every table column.
- `ModelCapabilityInput` is the write projection.
- `ModelCapabilityModel::upsert` is the only production SQL write entrance.
- `ModelCapabilityModel::get` reads the persisted projection.

The previous document described an unmigrated HuggingFace `models` table and a `ModelInfo` row.
That design never became runtime schema, so #621 removed its no-op implementation rather than
continuing to present it as a working database capability.
