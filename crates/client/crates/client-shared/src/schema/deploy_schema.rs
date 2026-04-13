use serde_json::json;

/// Deploy 表单的 JSON Schema 定义
pub fn deploy_schema() -> serde_json::Value {
    json!({
        "entity_type": "deploy",
        "label": "模型部署",
        "fields": [
            {
                "key": "source",
                "label": "来源",
                "type": "select",
                "required": true,
                "default": "HuggingFace",
                "options": [
                    {"value": "HuggingFace", "label": "HuggingFace"},
                    {"value": "Local", "label": "Local Path"}
                ]
            },
            {
                "key": "model_id",
                "label": "Model ID",
                "type": "text",
                "required": true,
                "placeholder": "e.g. gpt2 or organization/model"
            }
        ],
        "table_columns": [],
        "form_sections": [
            {"title": "部署配置", "fields": ["source", "model_id"]}
        ]
    })
}
