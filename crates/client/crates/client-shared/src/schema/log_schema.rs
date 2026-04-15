// JSON Schema builder — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// Log entity JSON Schema definition
pub fn log_schema() -> serde_json::Value {
    json!({
        "entity_type": "log",
        "label": "schema.log.label",
        "fields": [
            {
                "key": "id",
                "label": "ID",
                "type": "text",
                "visibility": "hidden"
            },
            {
                "key": "timestamp",
                "label": "schema.log.field.timestamp.label",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "level",
                "label": "schema.log.field.level.label",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "message",
                "label": "schema.log.field.message.label",
                "type": "text",
                "visibility": "table_only"
            }
        ],
        "table_columns": [
            {"key": "timestamp", "label": "schema.log.field.timestamp.label", "render": "monospace"},
            {"key": "level", "label": "schema.log.field.level.label", "render": "status_badge", "active_value": "INFO", "active_label": "INFO", "inactive_label": "OTHER"},
            {"key": "message", "label": "schema.log.field.message.label", "render": "text"}
        ],
        "form_sections": []
    })
}
