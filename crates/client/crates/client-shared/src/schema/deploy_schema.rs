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
                "key": "type",
                "label": "schema.deploy.field.type.label",
                "type": "select",
                "required": true,
                "default": "1",
                "options": [
                    {"value": "1", "label": "OpenAI"},
                    {"value": "14", "label": "Anthropic"},
                    {"value": "43", "label": "DeepSeek"},
                    {"value": "24", "label": "Gemini"},
                    {"value": "42", "label": "Mistral"},
                    {"value": "3", "label": "Azure"},
                    {"value": "4", "label": "Ollama"},
                    {"value": "20", "label": "OpenRouter"},
                    {"value": "40", "label": "SiliconFlow"},
                    {"value": "8", "label": "Custom"}
                ]
            },
            {
                "key": "model_id",
                "label": "Model ID",
                "type": "text",
                "required": true,
                "placeholder": "e.g. gpt-4o or organization/model"
            },
            {
                "key": "name",
                "label": "schema.deploy.field.name.label",
                "type": "text",
                "required": true,
                "placeholder": "e.g. My GPT-4o Channel"
            },
            {
                "key": "key",
                "label": "schema.deploy.field.key.label",
                "type": "password",
                "required": true,
                "placeholder": "API Key"
            },
            {
                "key": "group",
                "label": "schema.deploy.field.group.label",
                "type": "text",
                "required": true,
                "default": "default",
                "placeholder": "default"
            }
        ],
        "table_columns": [],
        "form_sections": [
            {"title": "schema.deploy.section.label", "fields": ["type", "model_id", "name", "key", "group"]}
        ]
    })
}
