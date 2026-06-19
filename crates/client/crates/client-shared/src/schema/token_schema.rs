// JSON schema definitions return serde_json::Value — these are UI form schemas,
// not domain data types.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// Token entity JSON Schema definition
pub fn token_schema() -> serde_json::Value {
    json!({
        "entity_type": "token",
        "label": "Token",
        "fields": [
            {
                "key": "token",
                "label": "Token",
                "type": "text",
                "required": false,
                "readonly": true,
                "visibility": "table_only"
            },
            {
                "key": "name",
                "label": "名称 / Name",
                "type": "text",
                "required": true,
                "placeholder": "e.g. My API Key",
                "visibility": "form_only"
            },
            {
                "key": "quota_limit",
                "label": "配额 / Quota",
                "type": "number",
                "required": false,
                "default": -1,
                "visibility": "both"
            },
            {
                "key": "used_quota",
                "label": "已用 / Used",
                "type": "number",
                "required": false,
                "readonly": true,
                "visibility": "table_only"
            },
            {
                "key": "status",
                "label": "状态 / Status",
                "type": "text",
                "required": false,
                "readonly": true,
                "visibility": "table_only"
            }
        ],
        "table_columns": [
            {"key": "token", "label": "Token", "render": "monospace"},
            {"key": "name", "label": "名称 / Name", "render": "text"},
            {"key": "status", "label": "状态 / Status", "render": "status_badge", "active_value": "active"},
            {"key": "used_quota", "label": "已用 / Used", "render": "text"},
            {"key": "quota_limit", "label": "配额 / Quota", "render": "text"}
        ],
        "form_sections": [
            {"title": "Token", "fields": ["name", "quota_limit"]}
        ]
    })
}
