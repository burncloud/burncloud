// JSON Schema builder — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// Login form JSON Schema definition
pub fn login_schema() -> serde_json::Value {
    json!({
        "entity_type": "login",
        "label": "schema.login.label",
        "fields": [
            {
                "key": "username",
                "label": "schema.login.field.username.label",
                "type": "text",
                "required": true,
                "placeholder": "schema.login.field.username.placeholder"
            },
            {
                "key": "password",
                "label": "schema.login.field.password.label",
                "type": "password",
                "required": true,
                "placeholder": "schema.login.field.password.placeholder"
            }
        ],
        "table_columns": [],
        "form_sections": [
            {"title": "schema.login.section.label", "fields": ["username", "password"]}
        ]
    })
}
