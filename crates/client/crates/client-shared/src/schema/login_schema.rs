// JSON Schema builder — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// Login 表单的 JSON Schema 定义
pub fn login_schema() -> serde_json::Value {
    json!({
        "entity_type": "login",
        "label": "登录",
        "fields": [
            {
                "key": "username",
                "label": "用户名",
                "type": "text",
                "required": true,
                "placeholder": "请输入用户名"
            },
            {
                "key": "password",
                "label": "密码",
                "type": "password",
                "required": true,
                "placeholder": "请输入密码"
            }
        ],
        "table_columns": [],
        "form_sections": [
            {"title": "登录", "fields": ["username", "password"]}
        ]
    })
}
