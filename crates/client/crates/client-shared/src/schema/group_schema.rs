use serde_json::json;

/// Group 实体的 JSON Schema 定义
pub fn group_schema() -> serde_json::Value {
    json!({
        "entity_type": "group",
        "label": "分组",
        "fields": [
            {
                "key": "id",
                "label": "ID",
                "type": "number",
                "visibility": "hidden"
            },
            {
                "key": "name",
                "label": "分组名称",
                "type": "text",
                "required": true,
                "placeholder": "e.g. production"
            },
            {
                "key": "strategy",
                "label": "负载均衡策略",
                "type": "select",
                "required": true,
                "default": "round_robin",
                "options": [
                    {"value": "round_robin", "label": "轮询 (Round Robin)"},
                    {"value": "weighted", "label": "加权 (Weighted)"}
                ]
            },
            {
                "key": "match_path",
                "label": "匹配路径",
                "type": "text",
                "default": "/v1/chat/completions",
                "placeholder": "/v1/chat/completions"
            }
        ],
        "table_columns": [
            {"key": "name", "label": "分组名称", "render": "text"},
            {"key": "match_path", "label": "匹配路径", "render": "monospace"},
            {"key": "strategy", "label": "策略", "render": "text"}
        ],
        "form_sections": [
            {"title": "创建分组", "fields": ["name", "strategy", "match_path"]}
        ]
    })
}
