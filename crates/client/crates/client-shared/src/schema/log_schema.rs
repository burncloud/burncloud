// JSON Schema builder — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// Log 实体的 JSON Schema 定义
pub fn log_schema() -> serde_json::Value {
    json!({
        "entity_type": "log",
        "label": "日志",
        "fields": [
            {
                "key": "id",
                "label": "ID",
                "type": "text",
                "visibility": "hidden"
            },
            {
                "key": "timestamp",
                "label": "时间",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "level",
                "label": "级别",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "message",
                "label": "消息",
                "type": "text",
                "visibility": "table_only"
            }
        ],
        "table_columns": [
            {"key": "timestamp", "label": "时间", "render": "monospace"},
            {"key": "level", "label": "级别", "render": "status_badge", "active_value": "INFO", "active_label": "INFO", "inactive_label": "OTHER"},
            {"key": "message", "label": "消息", "render": "text"}
        ],
        "form_sections": []
    })
}
