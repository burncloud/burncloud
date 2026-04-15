// JSON schema definitions return serde_json::Value — these are UI form schemas,
// not domain data types.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// Register form Schema
pub fn register_schema() -> serde_json::Value {
    json!({
        "entity_type": "register",
        "label": "schema.register.label",
        "fields": [
            {
                "key": "username",
                "label": "schema.register.field.username.label",
                "type": "text",
                "required": true,
                "placeholder": "schema.register.field.username.placeholder",
                "validation": [
                    {"rule": "min_length", "value": 3, "message": "schema.register.field.username.error.min_length"},
                    {"rule": "max_length", "value": 20, "message": "schema.register.field.username.error.max_length"},
                    {"rule": "pattern", "value": "^[a-zA-Z0-9_]+$", "message": "schema.register.field.username.error.pattern"}
                ]
            },
            {
                "key": "email",
                "label": "schema.register.field.email.label",
                "type": "text",
                "required": false,
                "placeholder": "schema.register.field.email.placeholder",
                "validation": [
                    {"rule": "email", "message": "schema.register.field.email.error.invalid"}
                ]
            },
            {
                "key": "password",
                "label": "schema.register.field.password.label",
                "type": "password",
                "required": true,
                "placeholder": "schema.register.field.password.placeholder",
                "validation": [
                    {"rule": "min_length", "value": 8, "message": "schema.register.field.password.error.min_length"}
                ]
            },
            {
                "key": "confirm_password",
                "label": "schema.register.field.confirm_password.label",
                "type": "password",
                "required": true,
                "placeholder": "schema.register.field.confirm_password.placeholder",
                "validation": [
                    {"rule": "match", "field": "password", "message": "schema.register.field.confirm_password.error.mismatch"}
                ]
            }
        ],
        "table_columns": [],
        "form_sections": [
            {"title": "schema.register.section.label", "fields": ["username", "email", "password", "confirm_password"]}
        ]
    })
}

/// User entity JSON Schema definition
pub fn user_schema() -> serde_json::Value {
    json!({
        "entity_type": "user",
        "label": "schema.user.label",
        "fields": [
            {
                "key": "id",
                "label": "ID",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "username",
                "label": "schema.user.field.username.label",
                "type": "text",
                "required": true
            },
            {
                "key": "role",
                "label": "schema.user.field.role.label",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "balance_cny",
                "label": "schema.user.field.balance_cny.label",
                "type": "number",
                "visibility": "table_only"
            },
            {
                "key": "balance_usd",
                "label": "schema.user.field.balance_usd.label",
                "type": "number",
                "visibility": "table_only"
            },
            {
                "key": "group",
                "label": "schema.user.field.group.label",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "status",
                "label": "schema.user.field.status.label",
                "type": "number",
                "visibility": "table_only"
            },
            {
                "key": "created_at",
                "label": "schema.user.field.created_at.label",
                "type": "text",
                "visibility": "table_only"
            }
        ],
        "table_columns": [
            {"key": "username", "label": "schema.user.field.username.label", "render": "text"},
            {"key": "role", "label": "schema.user.field.role.label", "render": "text"},
            {"key": "balance_cny", "label": "schema.user.field.balance_cny.label", "render": "money"},
            {"key": "status", "label": "schema.user.field.status.label", "render": "status_badge", "active_value": "1", "active_label": "Active", "inactive_label": "Disabled"}
        ],
        "form_sections": []
    })
}

/// Topup form Schema
pub fn topup_schema() -> serde_json::Value {
    json!({
        "entity_type": "topup",
        "label": "schema.topup.label",
        "fields": [
            {
                "key": "user_id",
                "label": "schema.topup.field.user_id.label",
                "type": "text",
                "required": true,
                "visibility": "hidden"
            },
            {
                "key": "amount",
                "label": "schema.topup.field.amount.label",
                "type": "number",
                "required": true,
                "placeholder": "100.00"
            }
        ],
        "table_columns": [],
        "form_sections": [
            {"title": "schema.topup.section.label", "fields": ["amount"]}
        ]
    })
}
