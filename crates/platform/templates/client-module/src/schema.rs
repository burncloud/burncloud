use serde_json::{json, Value};

/// JSON Schema 定义：{{entity_label}}
///
/// 包含 3 部分：
/// 1. fields: 表单字段（输入、选择、开关等）
/// 2. table_columns: 表格列定义（渲染、对齐、操作）
/// 3. form_sections: 页面表单的分组显示
pub fn get_schema() -> Value {
    json!({
        "entity_type": "{{crate_name}}",
        "label": "{{entity_label}}",
        "api_path": "{{api_path}}",
        "fields": [
            {
                "key": "id",
                "label": "ID",
                "type": "number",
                "default": 0,
                "visibility": "hidden"
            },
            {
                "key": "name",
                "label": "名称",
                "type": "text",
                "required": true,
                "placeholder": "请输入{{entity_label}}名称"
            },
            {
                "key": "description",
                "label": "描述",
                "type": "textarea",
                "placeholder": "关于{{entity_label}}的详细描述"
            },
            {
                "key": "status",
                "label": "状态",
                "type": "switch",
                "default": true
            }
        ],
        "table_columns": [
            {"key": "id", "label": "ID", "render": "text"},
            {"key": "name", "label": "名称", "render": "text"},
            {"key": "status", "label": "状态", "render": "status_badge", "active_value": true},
            {"key": "description", "label": "描述", "render": "text"}
        ],
        "form_sections": [
            {"title": "基本信息", "fields": ["name", "status"]},
            {"title": "详细描述", "fields": ["description"]}
        ]
    })
}
