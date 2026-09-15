//! OpenAPI documentation and Swagger UI integration.
//!
//! This module provides:
//! - `/api-docs/openapi.json` - OpenAPI 3.0 specification
//! - `/swagger-ui` - Interactive API documentation

use crate::AppState;
use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use serde::Serialize;
use std::env;

/// OpenAPI 3.0 specification structure
#[derive(Debug, Serialize)]
#[allow(clippy::disallowed_types)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: Info,
    pub paths: serde_json::Value,
    pub components: Components,
    pub servers: Vec<Server>,
}

#[derive(Debug, Serialize)]
pub struct Info {
    pub title: String,
    pub description: String,
    pub version: String,
    pub contact: Contact,
}

#[derive(Debug, Serialize)]
pub struct Contact {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct Components {
    pub security_schemes: SecuritySchemes,
    #[allow(clippy::disallowed_types)]
    pub schemas: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SecuritySchemes {
    #[serde(rename = "bearerAuth")]
    pub bearer_auth: BearerAuth,
}

#[derive(Debug, Serialize)]
pub struct BearerAuth {
    #[serde(rename = "type")]
    pub type_: String,
    pub scheme: String,
    pub bearer_format: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct Server {
    pub url: String,
    pub description: String,
}

/// Get the current version from Cargo.toml
fn get_version() -> String {
    env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string())
}

/// Generate the OpenAPI specification
#[allow(clippy::disallowed_types)]
fn generate_openapi_spec() -> OpenApiSpec {
    let version = get_version();

    OpenApiSpec {
        openapi: "3.0.3".to_string(),
        info: Info {
            title: "BurnCloud API".to_string(),
            description: "BurnCloud - AI-powered LLM API Gateway and Management Platform.\n\nA high-performance, low-latency API gateway for LLM services with enterprise-grade governance features including load balancing, billing, and multi-tenancy support.".to_string(),
            version,
            contact: Contact {
                name: "BurnCloud Team".to_string(),
                url: "https://github.com/burncloud/burncloud".to_string(),
            },
        },
        paths: generate_paths(),
        components: Components {
            security_schemes: SecuritySchemes {
                bearer_auth: BearerAuth {
                    type_: "http".to_string(),
                    scheme: "bearer".to_string(),
                    bearer_format: "JWT".to_string(),
                    description: "JWT Authorization header using the Bearer scheme. Example: \"Authorization: Bearer {token}\"".to_string(),
                },
            },
            schemas: generate_schemas(),
        },
        servers: vec![
            Server {
                url: "/".to_string(),
                description: "Current server".to_string(),
            },
        ],
    }
}

/// Generate API paths
#[allow(clippy::disallowed_types)]
fn generate_paths() -> serde_json::Value {
    serde_json::json!({
        "/api/auth/register": {
            "post": {
                "tags": ["Authentication"],
                "summary": "Register a new user",
                "description": "Create a new user account",
                "operationId": "registerUser",
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "$ref": "#/components/schemas/RegisterRequest"
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "User registered successfully",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/AuthResponse"
                                }
                            }
                        }
                    },
                    "400": {
                        "description": "Invalid request or username already exists",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ErrorResponse"
                                }
                            }
                        }
                    }
                }
            }
        },
        "/api/auth/login": {
            "post": {
                "tags": ["Authentication"],
                "summary": "Login user",
                "description": "Authenticate a user and return a JWT token",
                "operationId": "loginUser",
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "$ref": "#/components/schemas/LoginRequest"
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Login successful",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/AuthResponse"
                                }
                            }
                        }
                    },
                    "401": {
                        "description": "Invalid credentials",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ErrorResponse"
                                }
                            }
                        }
                    }
                }
            }
        },
        "/console/api/channel": {
            "get": {
                "tags": ["Channel Management"],
                "summary": "List channels",
                "description": "Get a paginated list of channels",
                "operationId": "listChannels",
                "security": [{"bearerAuth": []}],
                "parameters": [
                    {
                        "name": "limit",
                        "in": "query",
                        "description": "Maximum number of results to return",
                        "schema": {"type": "integer", "default": 20, "maximum": 100}
                    },
                    {
                        "name": "offset",
                        "in": "query",
                        "description": "Number of results to skip",
                        "schema": {"type": "integer", "default": 0}
                    }
                ],
                "responses": {
                    "200": {
                        "description": "List of channels",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ChannelListResponse"
                                }
                            }
                        }
                    }
                }
            },
            "post": {
                "tags": ["Channel Management"],
                "summary": "Create a channel",
                "description": "Create a new channel for LLM API routing",
                "operationId": "createChannel",
                "security": [{"bearerAuth": []}],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "$ref": "#/components/schemas/ChannelRequest"
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Channel created successfully",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ChannelCreatedResponse"
                                }
                            }
                        }
                    }
                }
            },
            "put": {
                "tags": ["Channel Management"],
                "summary": "Update a channel",
                "description": "Update an existing channel configuration",
                "operationId": "updateChannel",
                "security": [{"bearerAuth": []}],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "$ref": "#/components/schemas/ChannelRequest"
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Channel updated successfully",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ChannelResponse"
                                }
                            }
                        }
                    }
                }
            }
        },
        "/console/api/channel/{id}": {
            "get": {
                "tags": ["Channel Management"],
                "summary": "Get channel by ID",
                "description": "Retrieve a specific channel's configuration",
                "operationId": "getChannel",
                "security": [{"bearerAuth": []}],
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "description": "Channel ID",
                        "schema": {"type": "integer"}
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Channel details",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ChannelResponse"
                                }
                            }
                        }
                    },
                    "404": {
                        "description": "Channel not found",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ErrorResponse"
                                }
                            }
                        }
                    }
                }
            },
            "delete": {
                "tags": ["Channel Management"],
                "summary": "Delete a channel",
                "description": "Remove a channel from the system",
                "operationId": "deleteChannel",
                "security": [{"bearerAuth": []}],
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "description": "Channel ID",
                        "schema": {"type": "integer"}
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Channel deleted successfully"
                    },
                    "404": {
                        "description": "Channel not found",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ErrorResponse"
                                }
                            }
                        }
                    }
                }
            }
        },
        "/v1/chat/completions": {
            "post": {
                "tags": ["LLM API"],
                "summary": "Chat completions",
                "description": "Create a chat completion request. This endpoint is compatible with OpenAI's chat completions API.",
                "operationId": "chatCompletions",
                "security": [{"bearerAuth": []}],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "$ref": "#/components/schemas/ChatCompletionRequest"
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Chat completion response",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ChatCompletionResponse"
                                }
                            }
                        }
                    },
                    "400": {
                        "description": "Invalid request",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ErrorResponse"
                                }
                            }
                        }
                    },
                    "401": {
                        "description": "Unauthorized",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ErrorResponse"
                                }
                            }
                        }
                    },
                    "429": {
                        "description": "Rate limit exceeded",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ErrorResponse"
                                }
                            }
                        }
                    }
                }
            }
        },
        "/health": {
            "get": {
                "tags": ["System"],
                "summary": "Health check",
                "description": "Simple health check endpoint for load balancers and uptime monitors",
                "operationId": "healthCheck",
                "responses": {
                    "200": {
                        "description": "Service is healthy",
                        "content": {
                            "text/plain": {
                                "schema": {"type": "string", "example": "ok"}
                            }
                        }
                    }
                }
            }
        }
    })
}

/// Generate component schemas
#[allow(clippy::disallowed_types)]
fn generate_schemas() -> serde_json::Value {
    serde_json::json!({
        "RegisterRequest": {
            "type": "object",
            "required": ["username", "password"],
            "properties": {
                "username": {"type": "string", "minLength": 1},
                "password": {"type": "string", "minLength": 1},
                "email": {"type": "string", "format": "email"}
            }
        },
        "LoginRequest": {
            "type": "object",
            "required": ["username", "password"],
            "properties": {
                "username": {"type": "string"},
                "password": {"type": "string"}
            }
        },
        "AuthResponse": {
            "type": "object",
            "properties": {
                "success": {"type": "boolean"},
                "data": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "username": {"type": "string"},
                        "roles": {"type": "array", "items": {"type": "string"}},
                        "token": {"type": "string"}
                    }
                }
            }
        },
        "ErrorResponse": {
            "type": "object",
            "properties": {
                "success": {"type": "boolean", "example": false},
                "message": {"type": "string"}
            }
        },
        "ChannelRequest": {
            "type": "object",
            "required": ["type", "key", "name", "models", "group"],
            "properties": {
                "id": {"type": "integer"},
                "type": {"type": "integer", "description": "Channel type (1=OpenAI, 2=Claude, etc.)"},
                "key": {"type": "string", "description": "API key for the channel"},
                "name": {"type": "string", "description": "Human-readable channel name"},
                "base_url": {"type": "string", "description": "Base URL for API requests"},
                "models": {"type": "string", "description": "Comma-separated list of supported models"},
                "group": {"type": "string", "description": "Channel group for routing"},
                "weight": {"type": "integer", "default": 1, "description": "Load balancing weight"},
                "priority": {"type": "integer", "default": 0, "description": "Routing priority"},
                "rpm_cap": {"type": "integer", "description": "Requests per minute cap"},
                "tpm_cap": {"type": "integer", "description": "Tokens per minute cap"}
            }
        },
        "ChannelResponse": {
            "type": "object",
            "properties": {
                "success": {"type": "boolean"},
                "data": {"$ref": "#/components/schemas/Channel"}
            }
        },
        "Channel": {
            "type": "object",
            "properties": {
                "id": {"type": "integer"},
                "type": {"type": "integer"},
                "key": {"type": "string"},
                "name": {"type": "string"},
                "status": {"type": "integer"},
                "base_url": {"type": "string"},
                "models": {"type": "string"},
                "group": {"type": "string"},
                "weight": {"type": "integer"},
                "priority": {"type": "integer"}
            }
        },
        "ChannelListResponse": {
            "type": "object",
            "properties": {
                "success": {"type": "boolean"},
                "data": {
                    "type": "object",
                    "properties": {
                        "channels": {
                            "type": "array",
                            "items": {"$ref": "#/components/schemas/Channel"}
                        },
                        "pagination": {
                            "type": "object",
                            "properties": {
                                "limit": {"type": "integer"},
                                "offset": {"type": "integer"}
                            }
                        }
                    }
                }
            }
        },
        "ChannelCreatedResponse": {
            "type": "object",
            "properties": {
                "success": {"type": "boolean"},
                "data": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"}
                    }
                }
            }
        },
        "ChatCompletionRequest": {
            "type": "object",
            "required": ["model", "messages"],
            "properties": {
                "model": {"type": "string", "description": "Model ID to use"},
                "messages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "role": {"type": "string", "enum": ["system", "user", "assistant"]},
                            "content": {"type": "string"}
                        }
                    }
                },
                "temperature": {"type": "number", "minimum": 0, "maximum": 2},
                "max_tokens": {"type": "integer", "minimum": 1},
                "stream": {"type": "boolean", "default": false}
            }
        },
        "ChatCompletionResponse": {
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "object": {"type": "string", "example": "chat.completion"},
                "created": {"type": "integer"},
                "model": {"type": "string"},
                "choices": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "index": {"type": "integer"},
                            "message": {
                                "type": "object",
                                "properties": {
                                    "role": {"type": "string"},
                                    "content": {"type": "string"}
                                }
                            },
                            "finish_reason": {"type": "string"}
                        }
                    }
                },
                "usage": {
                    "type": "object",
                    "properties": {
                        "prompt_tokens": {"type": "integer"},
                        "completion_tokens": {"type": "integer"},
                        "total_tokens": {"type": "integer"}
                    }
                }
            }
        }
    })
}

/// Handler for OpenAPI JSON endpoint
async fn openapi_json() -> impl IntoResponse {
    let spec = generate_openapi_spec();
    axum::Json(spec)
}

/// Handler for Swagger UI
async fn swagger_ui() -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BurnCloud API Documentation</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
    <style>
        html { box-sizing: border-box; overflow: -moz-scrollbars-vertical; overflow-y: scroll; }
        *, *:before, *:after { box-sizing: inherit; }
        body { margin: 0; padding: 0; background: #fafafa; }
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {
            const ui = SwaggerUIBundle({
                url: "/api-docs/openapi.json",
                dom_id: '#swagger-ui',
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                layout: "StandaloneLayout",
                deepLinking: true,
                displayOperationId: false,
                defaultModelsExpandDepth: 1,
                defaultModelExpandDepth: 1,
                docExpansion: "list",
                syntaxHighlight: {
                    activate: true,
                    theme: "monokai"
                }
            });
            window.ui = ui;
        };
    </script>
</body>
</html>"#;

    Html(html)
}

/// Create the OpenAPI router
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api-docs/openapi.json", get(openapi_json))
        .route("/swagger-ui", get(swagger_ui))
        .route("/swagger-ui/", get(swagger_ui))
}
