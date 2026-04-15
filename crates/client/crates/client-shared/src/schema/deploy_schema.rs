// JSON Schema builder — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// Deploy form JSON Schema definition
pub fn deploy_schema() -> serde_json::Value {
    json!({
        "entity_type": "deploy",
        "label": "schema.deploy.label",
        "fields": [
            {
                "key": "source",
                "label": "schema.deploy.field.source.label",
                "type": "select",
                "required": true,
                "default": "HuggingFace",
                "options": [
                    {"value": "HuggingFace", "label": "HuggingFace"},
                    {"value": "Local", "label": "Local Path"}
                ]
            },
            {
                "key": "model_id",
                "label": "Model ID",
                "type": "text",
                "required": true,
                "placeholder": "e.g. gpt2 or organization/model"
            }
        ],
        "table_columns": [],
        "form_sections": [
            {"title": "schema.deploy.section.label", "fields": ["source", "model_id"]}
        ]
    })
}
