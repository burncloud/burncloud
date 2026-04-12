use serde_json::json;

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
