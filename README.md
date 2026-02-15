<div align="center">

# BurnCloud

![Rust](https://img.shields.io/badge/Built_with-Rust-orange?style=for-the-badge&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)
![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blue?style=for-the-badge)
![Tests](https://img.shields.io/badge/Tests-Passing-success?style=for-the-badge)

**The Next-Gen High-Performance AI Gateway & Aggregator**

[Feature Requests](https://github.com/burncloud/burncloud/issues) · [Roadmap](docs/ARCHITECTURE_EVOLUTION.md) · [Documentation](docs/)

[English](README.md) | [简体中文](README_CN.md)

</div>

---

## 💡 What is BurnCloud?

BurnCloud is a **Rust-native** LLM Aggregation Gateway and Management Platform.
It aims to benchmark against and surpass **One API (New API)**, providing individual developers, teams, and enterprises with a **high-performance, resource-efficient, secure, and controllable** unified LLM access layer.

**We are not just reinventing the wheel; we are upgrading the engine.**
If you are tired of the high memory consumption, GC pauses, or complex deployment dependencies of existing gateways, BurnCloud is your best choice.

## ✨ Why BurnCloud? (Core Values)

### 🚀 1. Performance First
*   **Powered by Rust**: Built on `Axum` and `Tokio`, offering astonishing concurrency handling capabilities and extremely low memory footprint (MB level vs GB level).
*   **Zero-Overhead Passthrough**: Featuring a unique "Don't Touch the Body" routing mode. In scenarios without protocol conversion, it achieves byte-level zero-copy forwarding with near-zero latency.
*   **Single Binary**: No Runtime dependencies (No Python, No Node.js, No Java). One file is a complete platform.

### 🔌 2. Universal Aggregation
*   **All to OpenAI**: Unifies protocols from Anthropic (Claude), Google (Gemini), Azure, Alibaba Qwen, and other mainstream models into standard **OpenAI format**.
*   **Write Once, Run Anywhere**: Your LangChain, AutoGPT, or any existing application can seamlessly switch underlying models just by changing the Base URL.

### ⚖️ 3. Enterprise Governance
*   **Smart Load Balancing**: Supports Multi-Channel Round-Robin, Weighted Distribution, and Automatic Failover. If one `gpt-4` goes down, thousands of `gpt-4` stand up.
*   **Precise Billing**: Supports precise token-based billing, custom Model Ratios, and User Group Ratios.
*   **Multi-Tenant Management**: Comprehensive redemption codes, quota management, and invitation mechanisms.

### 🛡️ 4. Rock-Solid Reliability
*   **Real-World E2E Testing**: We have abandoned fake Mock data. BurnCloud's CI/CD pipeline validates end-to-end against **real OpenAI/Gemini APIs**, ensuring core forwarding logic remains robust in real network environments.
*   **Browser-Driven Verification**: Built-in automated UI tests based on **Headless Chrome** ensure the rendering link from Backend API to Frontend Dioxus LiveView is unobstructed.
*   **Zero-Regression Promise**: Strict **"API-Path Matching"** testing strategy ensures every Commit passes rigorous automated auditing.

### 🎨 5. Fluent Experience
*   **More Than API**: Built-in local management client developed with **Dioxus**, featuring **Windows 11 Fluent Design**.
*   **Visual Monitoring**: View real-time TPS, RPM, and token consumption trends, saying goodbye to boring log files.

---

## 🏗️ Architecture

BurnCloud adopts a strict four-layer architecture to ensure high cohesion and low coupling:

*   **Gateway Layer (`crates/router`)**: Data plane. Handles high-concurrency traffic, authentication, rate limiting, and protocol conversion.
*   **Control Layer (`crates/server`)**: Control plane. Provides RESTful APIs for UI calls, managing configuration and state.
*   **Service Layer (`crates/service`)**: Business logic. Encapsulates core logic like billing, monitoring, and channel speed testing.
*   **Data Layer (`crates/database`)**: Data persistence. Based on SQLx + SQLite/PostgreSQL, with future Redis cache support.

> See: [Architecture Evolution](docs/ARCHITECTURE_EVOLUTION.md)

### Key Principle: "Don't Touch the Body"

The router is a smart pipe, not a processor. It handles authentication and routing but streams request/response bodies with zero latency.

---

## 🛠️ Getting Started

### Requirements
*   Rust 1.75+
*   Windows 10/11, Linux, or macOS

### Quick Start

```bash
# 1. Clone repository
git clone https://github.com/burncloud/burncloud.git
cd burncloud

# 2. Configure (Optional)
cp .env.example .env
# Edit .env and fill in TEST_OPENAI_KEY to enable full E2E tests

# 3. Build
cargo build --release

# 4. Run (Auto-compiles Server and Client)
cargo run                  # GUI on Windows, server with LiveView on Linux
cargo run -- router        # Server mode only
cargo run -- client        # GUI client only
```

### Configuration

Key configuration options:

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | Server port | 3000 |
| `HOST` | Server host | 0.0.0.0 |
| `DATABASE_URL` | Database connection | sqlite:burncloud.db |
| `RUST_LOG` | Log level | info |

### Basic Usage

Start the router:

```bash
cargo run -- router
```

Make a request:

```bash
curl http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### Run Tests (Quality Assurance)

Experience the industrial-grade testing process:

```bash
# Run all tests
cargo test --all-features

# Run all API integration tests
cargo test -p burncloud-tests --test api_tests

# Run UI automation tests (Requires Chrome)
cargo test -p burncloud-tests --test ui_tests

# Format check
cargo fmt --all -- --check

# Lint
cargo clippy --all-targets --all-features
```

---

## 💰 Billing & Quota

### Pricing Configuration

BurnCloud tracks token usage and calculates costs based on configurable pricing per model.

#### CLI Commands

List all model prices:

```bash
burncloud price list
```

Set price for a model (per 1 million tokens):

```bash
burncloud price set gpt-4 --input 30.0 --output 60.0
```

Get price for a specific model:

```bash
burncloud price get gpt-4
```

Delete a price:

```bash
burncloud price delete gpt-4
```

#### Pricing Format

Prices are defined per **1 million tokens**:

- `input_price`: Cost per 1M prompt tokens
- `output_price`: Cost per 1M completion tokens

Example calculation for GPT-4 with input=$30/1M, output=$60/1M:

```
For 100 prompt tokens + 200 completion tokens:
cost = (100/1,000,000 * 30) + (200/1,000,000 * 60)
     = 0.003 + 0.012
     = $0.015
```

#### Model Aliases

Models can be aliased to share pricing:

```bash
burncloud price set gpt-4-turbo --alias gpt-4
```

This makes `gpt-4-turbo` use the same pricing as `gpt-4`.

### Quota Management

Each token can have a quota limit. When a request is made:

1. System checks if token has sufficient remaining quota
2. Request is processed
3. Cost is calculated from token usage
4. Quota is deducted atomically

#### Quota Limits

- `quota_limit = -1`: Unlimited quota (default)
- `quota_limit >= 0`: Maximum usage allowed

#### Insufficient Quota Response

When quota is exhausted:

```json
{
  "error": {
    "message": "Insufficient quota",
    "type": "insufficient_quota_error",
    "code": "insufficient_quota"
  }
}
```

HTTP Status: `402 Payment Required`

### Token Expiry

Tokens can have an expiration time:

- `expired_time = -1`: Never expires (default)
- `expired_time > 0`: Unix timestamp of expiration

When a token expires:

```json
{
  "error": {
    "message": "Token has expired",
    "type": "invalid_request_error",
    "code": "token_expired"
  }
}
```

HTTP Status: `401 Unauthorized`

---

## 📊 Streaming Token Statistics

BurnCloud parses token usage from streaming responses for accurate billing.

### OpenAI

Enable usage stats in streaming:

```json
{
  "model": "gpt-4",
  "messages": [...],
  "stream": true,
  "stream_options": { "include_usage": true }
}
```

### Anthropic

Token counts are in `message_start` and `message_delta` events.

### Gemini

Token counts are in `usageMetadata` field.

---

## ⚖️ Load Balancing

### Weighted Random Selection

Channels can be assigned weights for traffic distribution:

- Weight 80/20: ~80% traffic to channel A, ~20% to channel B
- Weight 0: Falls back to round-robin

Weights are configured in the `abilities` table.

---

## ⚠️ Error Codes

BurnCloud returns errors in OpenAI-compatible format:

```json
{
  "error": {
    "message": "Error description",
    "type": "error_type",
    "code": "error_code"
  }
}
```

| HTTP Status | Code | Type | Description |
|-------------|------|------|-------------|
| 401 | `invalid_token` | `invalid_request_error` | Invalid or missing token |
| 401 | `token_expired` | `invalid_request_error` | Token has expired |
| 402 | `insufficient_quota` | `insufficient_quota_error` | Quota exceeded |
| 403 | `permission_denied` | `permission_error` | Permission denied |
| 404 | `not_found` | `not_found_error` | Resource not found |
| 429 | `rate_limit_exceeded` | `rate_limit_error` | Rate limited |
| 500 | `server_error` | `server_error` | Internal error |
| 503 | `service_unavailable` | `server_error` | Service unavailable |

---

## 🗺️ Roadmap

- [x] **v0.1**: Basic routing & AWS SigV4 signing support (Completed)
- [x] **v0.2**: Database integration, Basic Auth & **New API Core Replication** (Completed)
    - [x] Ability Smart Routing
    - [x] Channel Management API
    - [x] Async Billing & Logging
- [x] **v0.3**: Unified Protocol Adaptors (OpenAI/Gemini/Claude) & E2E Test Suite (Completed)
- [ ] **v0.4**: Smart Load Balancing & Failover (In Progress)
- [ ] **v0.5**: Web Console Frontend Polish
- [ ] **v1.0**: Official Release, Redis Cache Integration

---

## 🤝 Contributing

Contributions of any kind are welcome! Please read our **[Development Constitution](docs/CONSTITUTION.md)** before submitting code.

## 📄 License

MIT License © 2025 BurnCloud Team
