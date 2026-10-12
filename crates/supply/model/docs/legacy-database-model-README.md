# burncloud-database-model

Supply-owned persistence for model capability truth.

## Data truth

The runtime table is `model_capabilities`:

| Column | Meaning |
| --- | --- |
| `id` | persistence identifier |
| `model` | unique model name |
| `context_window` | maximum context tokens |
| `max_output_tokens` | maximum output tokens |
| `supports_vision` | vision input capability |
| `supports_function_calling` | function/tool calling capability |
| `input_price` / `output_price` | legacy USD price projection kept for schema compatibility |
| `synced_at` | last projection timestamp |

Canonical pricing remains in Commerce. The two price columns above are not a second pricing truth;
they remain only because the historical migration created a mixed table. A future data migration may
physically split them, but #621 does not rewrite migration history.

## Public API

- `ModelCapability` — row type matching the real table.
- `ModelCapabilityInput` — write projection accepted by the adapter.
- `ModelCapabilityModel::get` — read one row.
- `ModelCapabilityModel::upsert` — the single production write entrance.

The old HuggingFace-shaped `ModelInfo` / `ModelDatabase` API was removed. It described a different
unmigrated `models` design and all seven CRUD methods were no-ops.

## Boundary

Traffic may call this crate to persist a capability projection. It must not execute raw SQL against
`model_capabilities` itself.
