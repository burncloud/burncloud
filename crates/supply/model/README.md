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

The Cargo package is `burncloud-supply-model`. During #778 the Rust library
target intentionally remains `burncloud_service_models` so the structural
migration does not create unrelated import churn. A follow-up mechanical rename
will remove that compatibility surface.
