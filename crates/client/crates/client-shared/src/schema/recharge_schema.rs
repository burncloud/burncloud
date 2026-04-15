// JSON Schema builder — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use serde_json::json;

/// Recharge record entity JSON Schema definition
pub fn recharge_schema() -> serde_json::Value {
    json!({
        "entity_type": "recharge",
        "label": "schema.recharge.label",
        "fields": [
            {
                "key": "id",
                "label": "schema.recharge.field.id.label",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "created_at",
                "label": "schema.recharge.field.created_at.label",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "description",
                "label": "schema.recharge.field.description.label",
                "type": "text",
                "visibility": "table_only"
            },
            {
                "key": "amount",
                "label": "schema.recharge.field.amount.label",
                "type": "number",
                "visibility": "table_only"
            },
            {
                "key": "status",
                "label": "schema.recharge.field.status.label",
                "type": "text",
                "visibility": "table_only"
            }
        ],
        "table_columns": [
            {"key": "id", "label": "schema.recharge.field.id.label", "render": "monospace"},
            {"key": "created_at", "label": "schema.recharge.field.created_at.label", "render": "text"},
            {"key": "description", "label": "schema.recharge.field.description.label", "render": "text"},
            {"key": "amount", "label": "schema.recharge.field.amount.label", "render": "money"},
            {"key": "status", "label": "schema.recharge.field.status.label", "render": "status_badge", "active_value": "success", "active_label": "Success", "inactive_label": "Failed"}
        ],
        "form_sections": []
    })
}
