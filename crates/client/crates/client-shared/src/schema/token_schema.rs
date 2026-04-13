// JSON schema definitions return serde_json::Value — these are UI form schemas,
// not domain data types.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// Token 实体的 JSON Schema 定义
pub fn token_schema() -> serde_json::Value {
    json!({
        "entity_type": "token",
        "label": "令牌",
        "fields": [
            {
                "key": "token",
                "label": "令牌",
                "type": "text",
                "required": false,
                "readonly": true,
                "visibility": "table_only"
            },
            {
                "key": "user_id",
                "label": "用户标识 (User ID)",
                "type": "text",
                "required": true,
                "placeholder": "e.g. user-123",
                "visibility": "form_only"
            },
            {
                "key": "quota_limit",
                "label": "额度限制 (-1 无限)",
                "type": "number",
                "required": false,
                "default": -1,
                "visibility": "both"
            },
            {
                "key": "used_quota",
                "label": "已用额度",
                "type": "number",
                "required": false,
                "readonly": true,
                "visibility": "table_only"
            },
            {
                "key": "status",
                "label": "状态",
                "type": "text",
                "required": false,
                "readonly": true,
                "visibility": "table_only"
            }
        ],
        "table_columns": [
            {"key": "token", "label": "令牌", "render": "monospace"},
            {"key": "user_id", "label": "用户", "render": "text"},
            {"key": "status", "label": "状态", "render": "status_badge", "active_value": "active"},
            {"key": "used_quota", "label": "已用", "render": "text"},
            {"key": "quota_limit", "label": "额度", "render": "text"}
        ],
        "form_sections": [
            {"title": "生成新令牌", "fields": ["user_id", "quota_limit"]}
        ]
    })
}
