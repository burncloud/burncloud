use serde_json::json;

/// Recharge（充值记录）实体的 JSON Schema 定义
pub fn recharge_schema() -> serde_json::Value {
    json!({
        "entity_type": "recharge",
        "label": "充值记录",
        "fields": [
            {
                "key": "id",
                "label": "交易 ID",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "created_at",
                "label": "时间",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "description",
                "label": "描述",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "amount",
                "label": "金额",
                "type": "number",
                "visibility": "table_only"
            },
            {
                "key": "status",
                "label": "状态",
                "type": "text",
                "visibility": "table_only"
            }
        ],
        "table_columns": [
            {"key": "id", "label": "交易 ID", "render": "monospace"},
            {"key": "created_at", "label": "时间", "render": "text"},
            {"key": "description", "label": "描述", "render": "text"},
            {"key": "amount", "label": "金额", "render": "money"},
            {"key": "status", "label": "状态", "render": "status_badge", "active_value": "success", "active_label": "成功", "inactive_label": "失败"}
        ],
        "form_sections": []
    })
}
