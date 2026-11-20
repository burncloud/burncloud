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
        },
        "/api/node/hardware": {
            "get": {
                "tags": ["Node"],
                "summary": "Get the server hardware snapshot",
                "description": "Read the current Node host CPU, physical memory, filesystem capacity and detected accelerators. GPU driver failures degrade to an empty or partial accelerator list.",
                "operationId": "getNodeHardware",
                "security": [],
                "responses": {
                    "200": {
                        "description": "Hardware snapshot",
                        "headers": {
                            "Cache-Control": {
                                "description": "The response is not cacheable.",
                                "schema": {"type": "string", "example": "no-store"}
                            }
                        },
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/HardwareResponse"}
                            }
                        }
                    },
                    "501": {
                        "description": "The current platform has no hardware probe implementation",
                        "headers": {
                            "Cache-Control": {
                                "description": "The response is not cacheable.",
                                "schema": {"type": "string", "example": "no-store"}
                            }
                        },
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/HardwareErrorResponse"}
                            }
                        }
                    },
                    "500": {
                        "description": "The common hardware facts could not be read",
                        "headers": {
                            "Cache-Control": {
                                "description": "The response is not cacheable.",
                                "schema": {"type": "string", "example": "no-store"}
                            }
                        },
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/HardwareErrorResponse"}
                            }
                        }
                    }
                }
            }
        },
        "/api/node/models/resolve": {
            "post": {
                "tags": ["Node"],
                "summary": "Resolve model variants against the server hardware",
                "description": "Discover GGUF artifacts for the selected external repository model, probe the current Node host and return every compatible model variant. The client must provide the selected model, source and revision.",
                "operationId": "resolveNodeModel",
                "security": [],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {"$ref": "#/components/schemas/ResolveModelRequest"}
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Model resolution completed, including the normal unsupported outcome",
                        "headers": {
                            "Cache-Control": {
                                "description": "The response is not cacheable.",
                                "schema": {"type": "string", "example": "no-store"}
                            }
                        },
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/ResolveModelResponse"}
                            }
                        }
                    },
                    "400": {
                        "description": "The model request is invalid",
                        "headers": {
                            "Cache-Control": {
                                "description": "The response is not cacheable.",
                                "schema": {"type": "string", "example": "no-store"}
                            }
                        },
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/ModelResolutionErrorResponse"}
                            }
                        }
                    },
                    "404": {
                        "description": "The selected external model repository or revision was not found",
                        "headers": {
                            "Cache-Control": {
                                "description": "The response is not cacheable.",
                                "schema": {"type": "string", "example": "no-store"}
                            }
                        },
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/ModelResolutionErrorResponse"}
                            }
                        }
                    },
                    "501": {
                        "description": "The current platform has no hardware probe implementation",
                        "headers": {
                            "Cache-Control": {
                                "description": "The response is not cacheable.",
                                "schema": {"type": "string", "example": "no-store"}
                            }
                        },
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/ModelResolutionErrorResponse"}
                            }
                        }
                    },
                    "500": {
                        "description": "External artifact discovery, hardware probing or model resolution failed",
                        "headers": {
                            "Cache-Control": {
                                "description": "The response is not cacheable.",
                                "schema": {"type": "string", "example": "no-store"}
                            }
                        },
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/ModelResolutionErrorResponse"}
                            }
                        }
                    }
                }
            }
        },
        "/api/node/models/download": {
            "post": {
                "tags": ["Node"],
                "summary": "Download and verify a selected model artifact",
                "description": "Download the selected GGUF artifact on the Node host through the persistent download manager and verify its SHA-256 checksum. The operation waits for completion and is idempotent for the same artifact identity.",
                "operationId": "downloadNodeModel",
                "security": [],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {"$ref": "#/components/schemas/DownloadModelRequest"}
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "The artifact was downloaded or reused and verified",
                        "headers": {
                            "Cache-Control": {
                                "description": "The response is not cacheable.",
                                "schema": {"type": "string", "example": "no-store"}
                            }
                        },
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/DownloadModelResponse"}
                            }
                        }
                    },
                    "400": {
                        "description": "The download request or artifact address is invalid",
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/ModelDownloadErrorResponse"}
                            }
                        }
                    },
                    "422": {
                        "description": "The downloaded file does not match the requested checksum",
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/ModelDownloadErrorResponse"}
                            }
                        }
                    },
                    "500": {
                        "description": "The downloaded file could not be read or verified",
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/ModelDownloadErrorResponse"}
                            }
                        }
                    },
                    "502": {
                        "description": "The upstream artifact download failed",
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/ModelDownloadErrorResponse"}
                            }
                        }
                    },
                    "503": {
                        "description": "The download manager could not be initialized",
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/ModelDownloadErrorResponse"}
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
        "HardwareResponse": {
            "type": "object",
            "required": ["success", "data"],
            "properties": {
                "success": {"type": "boolean", "example": true},
                "data": {"$ref": "#/components/schemas/HardwareSnapshot"}
            }
        },
        "HardwareSnapshot": {
            "type": "object",
            "required": ["platform", "arch", "cpu", "memory_bytes", "disk_available_bytes", "accelerators"],
            "properties": {
                "platform": {"type": "string", "enum": ["linux", "macos", "windows", "other"]},
                "arch": {"type": "string", "example": "x86_64"},
                "cpu": {"type": "string", "example": "AMD Ryzen 9 7950X"},
                "memory_bytes": {"type": "integer", "format": "uint64", "description": "Total physical memory in bytes"},
                "disk_available_bytes": {"type": "integer", "format": "uint64", "description": "Available bytes on the filesystem containing the server working directory"},
                "accelerators": {
                    "type": "array",
                    "items": {"$ref": "#/components/schemas/AcceleratorSnapshot"}
                }
            }
        },
        "AcceleratorSnapshot": {
            "type": "object",
            "required": ["kind", "name", "memory_bytes"],
            "properties": {
                "kind": {"type": "string", "enum": ["nvidia", "amd", "intel", "apple", "other"]},
                "name": {"type": "string"},
                "memory_bytes": {"type": "integer", "format": "uint64", "nullable": true, "description": "Accelerator memory capacity in bytes; null means unknown"}
            }
        },
        "HardwareErrorResponse": {
            "type": "object",
            "required": ["success", "error"],
            "properties": {
                "success": {"type": "boolean", "example": false},
                "error": {
                    "type": "object",
                    "required": ["code", "message"],
                    "properties": {
                        "code": {"type": "string", "enum": ["unsupported_platform", "hardware_probe_failed"]},
                        "message": {"type": "string"}
                    }
                }
            }
        },
        "ResolveModelRequest": {
            "type": "object",
            "required": ["model", "source", "revision"],
            "properties": {
                "model": {
                    "type": "string",
                    "minLength": 3,
                    "description": "External repository identity in owner/name form.",
                    "example": "Qwen/Qwen3-8B"
                },
                "source": {
                    "type": "string",
                    "enum": ["modelscope", "hf-mirror", "huggingface"],
                    "description": "Repository selected by the model catalog.",
                    "example": "modelscope"
                },
                "revision": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Repository branch, tag or revision selected by the catalog.",
                    "example": "master"
                }
            }
        },
        "ResolveModelResponse": {
            "type": "object",
            "required": ["success", "data"],
            "properties": {
                "success": {"type": "boolean", "example": true},
                "data": {"$ref": "#/components/schemas/ResolveModelData"}
            }
        },
        "ResolveModelData": {
            "type": "object",
            "required": ["status", "model", "hardware", "candidates"],
            "properties": {
                "status": {"type": "string", "enum": ["resolved", "unsupported"]},
                "model": {"type": "string", "example": "Qwen/Qwen3-8B"},
                "hardware": {"$ref": "#/components/schemas/HardwareSnapshot"},
                "candidates": {
                    "type": "array",
                    "items": {"$ref": "#/components/schemas/ModelCandidate"}
                },
                "reason": {"$ref": "#/components/schemas/ResolveReason"}
            }
        },
        "ModelCandidate": {
            "type": "object",
            "required": ["model_name", "variant_id", "quantization", "min_ram_bytes", "min_vram_bytes", "disk_size_bytes", "runtime", "artifact_uri", "checksum"],
            "properties": {
                "model_name": {"type": "string"},
                "variant_id": {"type": "string"},
                "quantization": {"type": "string"},
                "min_ram_bytes": {"type": "integer", "format": "uint64"},
                "min_vram_bytes": {"type": "integer", "format": "uint64"},
                "disk_size_bytes": {"type": "integer", "format": "uint64"},
                "runtime": {"type": "string", "example": "llama.cpp"},
                "artifact_uri": {"type": "string", "format": "uri"},
                "checksum": {"type": "string", "example": "sha256:..."}
            }
        },
        "ResolveReason": {
            "type": "object",
            "required": ["code", "message"],
            "properties": {
                "code": {"type": "string", "enum": ["no_compatible_variant"]},
                "message": {"type": "string"}
            }
        },
        "ModelResolutionErrorResponse": {
            "type": "object",
            "required": ["success", "error"],
            "properties": {
                "success": {"type": "boolean", "example": false},
                "error": {
                    "type": "object",
                    "required": ["code", "message"],
                    "properties": {
                        "code": {"type": "string", "enum": ["invalid_model", "model_repository_not_found", "model_repository_unavailable", "unsupported_platform", "hardware_probe_failed", "manifest_invalid", "model_resolution_failed"]},
                        "message": {"type": "string"}
                    }
                }
            }
        },
        "DownloadModelRequest": {
            "type": "object",
            "required": ["model", "source", "revision", "variant_id", "artifact_uri", "checksum"],
            "properties": {
                "model": {"type": "string", "example": "Qwen/Qwen3-8B"},
                "source": {"type": "string", "enum": ["modelscope", "hf-mirror", "huggingface"]},
                "revision": {"type": "string", "example": "master"},
                "variant_id": {"type": "string", "example": "Q4_K_M.gguf"},
                "artifact_uri": {"type": "string", "format": "uri", "example": "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Q4_K_M.gguf"},
                "checksum": {"type": "string", "pattern": "^sha256:[0-9a-fA-F]{64}$"}
            }
        },
        "DownloadModelResponse": {
            "type": "object",
            "required": ["success", "data"],
            "properties": {
                "success": {"type": "boolean", "example": true},
                "data": {"$ref": "#/components/schemas/DownloadModelData"}
            }
        },
        "DownloadModelData": {
            "type": "object",
            "required": ["status", "model", "source", "revision", "variant_id", "local_path", "actual_sha256"],
            "properties": {
                "status": {"type": "string", "enum": ["verified"]},
                "model": {"type": "string"},
                "source": {"type": "string", "enum": ["modelscope", "hf-mirror", "huggingface"]},
                "revision": {"type": "string"},
                "variant_id": {"type": "string"},
                "local_path": {"type": "string", "description": "Path on the Node host; it is not a client-local path."},
                "actual_sha256": {"type": "string", "pattern": "^sha256:[0-9a-fA-F]{64}$"}
            }
        },
        "ModelDownloadErrorResponse": {
            "type": "object",
            "required": ["success", "error"],
            "properties": {
                "success": {"type": "boolean", "example": false},
                "error": {
                    "type": "object",
                    "required": ["code", "message"],
                    "properties": {
                        "code": {"type": "string", "enum": ["invalid_download_request", "checksum_mismatch", "download_failed", "download_service_unavailable", "download_storage_failed"]},
                        "message": {"type": "string"}
                    }
                }
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

#[cfg(test)]
#[path = "openapi_tests.rs"]
mod tests;

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
