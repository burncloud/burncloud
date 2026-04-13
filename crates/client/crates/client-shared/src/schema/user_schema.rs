// JSON schema definitions return serde_json::Value — these are UI form schemas,
// not domain data types.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// 注册表单 Schema
pub fn register_schema() -> serde_json::Value {
    json!({
        "entity_type": "register",
        "label": "注册",
        "fields": [
            {
                "key": "username",
                "label": "用户名",
                "type": "text",
                "required": true,
                "placeholder": "设置您的唯一标识",
                "validation": [
                    {"rule": "min_length", "value": 3, "message": "用户名至少需要3个字符"},
                    {"rule": "max_length", "value": 20, "message": "用户名不能超过20个字符"},
                    {"rule": "pattern", "value": "^[a-zA-Z0-9_]+$", "message": "用户名只能包含字母、数字和下划线"}
                ]
            },
            {
                "key": "email",
                "label": "邮箱",
                "type": "text",
                "required": false,
                "placeholder": "用于接收通知 (可选)",
                "validation": [
                    {"rule": "email", "message": "邮箱格式不正确"}
                ]
            },
            {
                "key": "password",
                "label": "密码",
                "type": "password",
                "required": true,
                "placeholder": "设置强密码",
                "validation": [
                    {"rule": "min_length", "value": 8, "message": "密码至少需要8个字符"}
                ]
            },
            {
                "key": "confirm_password",
                "label": "确认密码",
                "type": "password",
                "required": true,
                "placeholder": "再次输入密码",
                "validation": [
                    {"rule": "match", "field": "password", "message": "两次输入的密码不一致"}
                ]
            }
        ],
        "table_columns": [],
        "form_sections": [
            {"title": "注册信息", "fields": ["username", "email", "password", "confirm_password"]}
        ]
    })
}

/// User 实体的 JSON Schema 定义
pub fn user_schema() -> serde_json::Value {
    json!({
        "entity_type": "user",
        "label": "用户",
        "fields": [
            {
                "key": "id",
                "label": "ID",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "username",
                "label": "用户名",
                "type": "text",
                "required": true
            },
            {
                "key": "role",
                "label": "角色",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "balance_cny",
                "label": "CNY 余额",
                "type": "number",
                "visibility": "table_only"
            },
            {
                "key": "balance_usd",
                "label": "USD 余额",
                "type": "number",
                "visibility": "table_only"
            },
            {
                "key": "group",
                "label": "分组",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "status",
                "label": "状态",
                "type": "number",
                "visibility": "table_only"
            },
            {
                "key": "created_at",
                "label": "创建时间",
                "type": "text",
                "visibility": "table_only"
            }
        ],
        "table_columns": [
            {"key": "username", "label": "客户信息", "render": "text"},
            {"key": "role", "label": "角色", "render": "text"},
            {"key": "balance_cny", "label": "账户余额", "render": "money"},
            {"key": "status", "label": "状态", "render": "status_badge", "active_value": "1", "active_label": "正常", "inactive_label": "已禁用"}
        ],
        "form_sections": []
    })
}

/// Topup（充值）表单的 Schema
pub fn topup_schema() -> serde_json::Value {
    json!({
        "entity_type": "topup",
        "label": "充值",
        "fields": [
            {
                "key": "user_id",
                "label": "用户 ID",
                "type": "text",
                "required": true,
                "visibility": "hidden"
            },
            {
                "key": "amount",
                "label": "充值金额 (CNY)",
                "type": "number",
                "required": true,
                "placeholder": "100.00"
            }
        ],
        "table_columns": [],
        "form_sections": [
            {"title": "充值", "fields": ["amount"]}
        ]
    })
}
