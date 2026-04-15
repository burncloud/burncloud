// JSON schema definitions return serde_json::Value — these are UI form schemas,
// not domain data types.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// Token entity JSON Schema definition
pub fn token_schema() -> serde_json::Value {
    json!({
        "entity_type": "token",
        "label": "schema.token.label",
        "fields": [
            {
                "key": "token",
                "label": "schema.token.field.token.label",
                "type": "text",
                "required": false,
                "readonly": true,
                "visibility": "table_only"
            },
            {
                "key": "user_id",
                "label": "schema.token.field.user_id.label",
                "type": "text",
                "required": true,
                "placeholder": "e.g. user-123",
                "visibility": "form_only"
            },
            {
                "key": "quota_limit",
                "label": "schema.token.field.quota_limit.label",
                "type": "number",
                "required": false,
                "default": -1,
                "visibility": "both"
            },
            {
                "key": "used_quota",
                "label": "schema.token.field.used_quota.label",
                "type": "number",
                "required": false,
                "readonly": true,
                "visibility": "table_only"
            },
            {
                "key": "status",
                "label": "schema.token.field.status.label",
                "type": "text",
                "required": false,
                "readonly": true,
                "visibility": "table_only"
            }
        ],
        "table_columns": [
            {"key": "token", "label": "schema.token.field.token.label", "render": "monospace"},
            {"key": "user_id", "label": "schema.token.field.user_id.label", "render": "text"},
            {"key": "status", "label": "schema.token.field.status.label", "render": "status_badge", "active_value": "active"},
            {"key": "used_quota", "label": "schema.token.field.used_quota.label", "render": "text"},
            {"key": "quota_limit", "label": "schema.token.field.quota_limit.label", "render": "text"}
        ],
        "form_sections": [
            {"title": "schema.token.section.label", "fields": ["user_id", "quota_limit"]}
        ]
    })
}
