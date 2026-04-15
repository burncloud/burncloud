// JSON schema definitions return serde_json::Value — these are UI form schemas,
// not domain data types.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// Group entity JSON Schema definition
pub fn group_schema() -> serde_json::Value {
    json!({
        "entity_type": "group",
        "label": "schema.group.label",
        "fields": [
            {
                "key": "id",
                "label": "ID",
                "type": "number",
                "visibility": "hidden"
            },
            {
                "key": "name",
                "label": "schema.group.field.name.label",
                "type": "text",
                "required": true,
                "placeholder": "e.g. production"
            },
            {
                "key": "strategy",
                "label": "schema.group.field.strategy.label",
                "type": "select",
                "required": true,
                "default": "round_robin",
                "options": [
                    {"value": "round_robin", "label": "Round Robin"},
                    {"value": "weighted", "label": "Weighted"}
                ]
            },
            {
                "key": "match_path",
                "label": "schema.group.field.match_path.label",
                "type": "text",
                "default": "/v1/chat/completions",
                "placeholder": "/v1/chat/completions"
            }
        ],
        "table_columns": [
            {"key": "name", "label": "schema.group.field.name.label", "render": "text"},
            {"key": "match_path", "label": "schema.group.field.match_path.label", "render": "monospace"},
            {"key": "strategy", "label": "schema.group.field.strategy.label", "render": "text"}
        ],
        "form_sections": [
            {"title": "schema.group.section.label", "fields": ["name", "strategy", "match_path"]}
        ]
    })
}
