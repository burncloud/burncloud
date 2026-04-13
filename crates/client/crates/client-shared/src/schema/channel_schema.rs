use serde_json::json;

/// Channel 实体的 JSON Schema 定义
///
/// 包含 6 种供应商类型的条件字段：
/// - OpenAI (type=1): API Key
/// - Anthropic (type=14): API Key
/// - Google Gemini (type=24): API Key 或 Vertex AI Service Account
/// - AWS Bedrock (type=99): Access Key ID, Secret Key, Region, Model ID
/// - Azure OpenAI (type=98): Resource Name, Deployment, API Key, API Version
/// - Local / GGUF (type=97): Server URL
pub fn channel_schema() -> serde_json::Value {
    json!({
        "entity_type": "channel",
        "label": "渠道",
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
                "label": "连接名称",
                "type": "text",
                "required": true
            },
            {
                "key": "type",
                "label": "供应商类型",
                "type": "select",
                "default": "1",
                "visibility": "hidden",
                "options": [
                    {"value": "1", "label": "OpenAI"},
                    {"value": "14", "label": "Anthropic"},
                    {"value": "24", "label": "Google Gemini"},
                    {"value": "99", "label": "AWS Bedrock"},
                    {"value": "98", "label": "Azure OpenAI"},
                    {"value": "97", "label": "Local / GGUF"}
                ]
            },
            {
                "key": "status",
                "label": "状态",
                "type": "number",
                "default": 1,
                "visibility": "hidden"
            },
            {
                "key": "base_url",
                "label": "Base URL",
                "type": "text",
                "visibility": "hidden"
            },
            {
                "key": "models",
                "label": "模型",
                "type": "text",
                "visibility": "hidden"
            },
            {
                "key": "group",
                "label": "分组",
                "type": "text",
                "default": "default",
                "visibility": "hidden"
            },
            {
                "key": "priority",
                "label": "优先级",
                "type": "number",
                "default": 0,
                "visibility": "hidden"
            },
            {
                "key": "weight",
                "label": "权重",
                "type": "number",
                "default": 0,
                "visibility": "hidden"
            },

            // OpenAI / Anthropic: API Key
            {
                "key": "key",
                "label": "API Key",
                "type": "password",
                "required": true,
                "placeholder": "sk-...",
                "visible_when": {"or": [
                    {"field": "type", "in": ["1"]},
                    {"field": "type", "in": ["14"]}
                ]}
            },

            // Google: Auth Type 选择
            {
                "key": "google_auth_type",
                "label": "认证类型",
                "type": "select",
                "default": "api_key",
                "options": [
                    {"value": "api_key", "label": "Gemini API"},
                    {"value": "vertex", "label": "Vertex AI"}
                ],
                "visible_when": {"field": "type", "in": ["24"]}
            },
            // Google: API Key (api_key mode)
            {
                "key": "google_key",
                "label": "API Key",
                "type": "password",
                "placeholder": "AIza...",
                "visible_when": {"and": [
                    {"field": "type", "in": ["24"]},
                    {"field": "google_auth_type", "in": ["api_key"]}
                ]}
            },
            // Google: Vertex Service Account Key (vertex mode)
            {
                "key": "google_vertex_key",
                "label": "Service Account JSON Key",
                "type": "textarea",
                "placeholder": "{ \"type\": \"service_account\", \"project_id\": ... }",
                "visible_when": {"and": [
                    {"field": "type", "in": ["24"]},
                    {"field": "google_auth_type", "in": ["vertex"]}
                ]}
            },
            // Google: Region (vertex mode)
            {
                "key": "google_region",
                "label": "区域 (Region)",
                "type": "select",
                "default": "us-central1",
                "options": [
                    {"value": "us-central1", "label": "US Central (Iowa)"},
                    {"value": "us-east4", "label": "US East (N. Virginia)"},
                    {"value": "us-west1", "label": "US West (Oregon)"},
                    {"value": "asia-northeast1", "label": "Asia (Tokyo)"},
                    {"value": "asia-southeast1", "label": "Asia (Singapore)"},
                    {"value": "europe-west1", "label": "Europe (Belgium)"}
                ],
                "visible_when": {"and": [
                    {"field": "type", "in": ["24"]},
                    {"field": "google_auth_type", "in": ["vertex"]}
                ]}
            },
            // Google: Project ID (vertex mode)
            {
                "key": "google_project_id",
                "label": "Project ID",
                "type": "text",
                "placeholder": "Override JSON project_id",
                "visible_when": {"and": [
                    {"field": "type", "in": ["24"]},
                    {"field": "google_auth_type", "in": ["vertex"]}
                ]}
            },

            // AWS Bedrock
            {
                "key": "aws_key",
                "label": "Access Key ID",
                "type": "text",
                "required": true,
                "placeholder": "AKIA...",
                "visible_when": {"field": "type", "in": ["99"]}
            },
            {
                "key": "aws_sk",
                "label": "Secret Access Key",
                "type": "password",
                "required": true,
                "placeholder": "wJalrX...",
                "visible_when": {"field": "type", "in": ["99"]}
            },
            {
                "key": "aws_region",
                "label": "区域 (Region)",
                "type": "select",
                "default": "us-east-1",
                "options": [
                    {"value": "us-east-1", "label": "US East (N. Virginia)"},
                    {"value": "us-west-2", "label": "US West (Oregon)"},
                    {"value": "ap-northeast-1", "label": "Asia Pacific (Tokyo)"},
                    {"value": "eu-central-1", "label": "Europe (Frankfurt)"}
                ],
                "visible_when": {"field": "type", "in": ["99"]}
            },
            {
                "key": "aws_model_id",
                "label": "Model ID",
                "type": "text",
                "default": "anthropic.claude-sonnet-4-5-20250929-v1:0",
                "placeholder": "anthropic.claude-sonnet-4-5...",
                "visible_when": {"field": "type", "in": ["99"]}
            },

            // Azure OpenAI
            {
                "key": "azure_resource",
                "label": "Resource Name",
                "type": "text",
                "required": true,
                "placeholder": "my-openai-resource",
                "visible_when": {"field": "type", "in": ["98"]}
            },
            {
                "key": "azure_deployment",
                "label": "Deployment Name",
                "type": "text",
                "required": true,
                "placeholder": "gpt-4-deployment",
                "visible_when": {"field": "type", "in": ["98"]}
            },
            {
                "key": "azure_key",
                "label": "API Key",
                "type": "password",
                "required": true,
                "placeholder": "32-char hex string",
                "visible_when": {"field": "type", "in": ["98"]}
            },
            {
                "key": "azure_api_version",
                "label": "API Version",
                "type": "select",
                "default": "2023-05-15",
                "options": [
                    {"value": "2023-05-15", "label": "2023-05-15"},
                    {"value": "2023-12-01-preview", "label": "2023-12-01-preview"},
                    {"value": "2024-02-15-preview", "label": "2024-02-15-preview"}
                ],
                "visible_when": {"field": "type", "in": ["98"]}
            },

            // Local / GGUF
            {
                "key": "local_url",
                "label": "Local Server URL",
                "type": "text",
                "required": true,
                "default": "http://localhost:8080",
                "placeholder": "http://localhost:8080",
                "visible_when": {"field": "type", "in": ["97"]}
            }
        ],
        "table_columns": [
            {"key": "status", "label": "Status", "render": "status_badge", "active_value": "1", "active_label": "Running", "inactive_label": "Stopped"},
            {"key": "name", "label": "Name", "render": "text"},
            {"key": "models", "label": "Models", "render": "tags"}
        ],
        "form_sections": [
            {"title": "基本信息", "fields": ["name"]},
            {"title": "认证配置", "fields": [
                "key", "google_auth_type", "google_key", "google_vertex_key",
                "google_region", "google_project_id",
                "aws_key", "aws_sk", "aws_region", "aws_model_id",
                "azure_resource", "azure_deployment", "azure_key", "azure_api_version",
                "local_url"
            ]}
        ]
    })
}
