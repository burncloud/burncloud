# burncloud-supply-model

Supply-owned vertical Model crate.

This crate consolidates the former `burncloud-database-model` and
`burncloud-service-models` technical layers under one business Owner.

It owns:

- `model_capabilities` capability persistence defined by #621;
- Model manifest and variant structures;
- Model resolver interfaces and current resolver implementations;
- HuggingFace discovery/file traversal helpers;
- model download URL/data-directory helpers.

It does not own canonical pricing truth (Commerce), Traffic routing, database
schema migrations, or runtime process lifecycle.

The Cargo package is `burncloud-supply-model`, and the Rust crate import is
`burncloud_supply_model`. The temporary `burncloud_service_models`
compatibility surface used during #778 was removed by #780.
