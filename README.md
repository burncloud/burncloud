# BurnCloud

A Rust-native LLM Aggregation Gateway with built-in billing, quota management, and multi-provider support.

## Features

- **Multi-Provider Support**: OpenAI, Anthropic (Claude), Google Gemini, AWS Bedrock, Azure OpenAI
- **Streaming Support**: Zero-latency streaming passthrough with token counting
- **Billing & Quota**: Per-token usage tracking, cost calculation, and quota enforcement
- **Weighted Load Balancing**: Distribute traffic across channels with configurable weights
- **Token Management**: API keys with expiry dates and usage limits
- **Protocol Adaptation**: Seamless conversion between OpenAI, Anthropic, and Gemini APIs

## Quick Start

### Installation

```bash
# Build from source
cargo build --release

# Or run directly
cargo run -- router
```

### Configuration

Copy the example environment file:

```bash
cp .env.example .env
```

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

## Billing & Quota

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

## Streaming Token Statistics

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

## Load Balancing

### Weighted Random Selection

Channels can be assigned weights for traffic distribution:

- Weight 80/20: ~80% traffic to channel A, ~20% to channel B
- Weight 0: Falls back to round-robin

Weights are configured in the `abilities` table.

## Error Codes

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

## Architecture

BurnCloud follows a four-layer architecture:

1. **Gateway (`crates/router`)**: Data plane for high-concurrency traffic
2. **Control (`crates/server`)**: Control plane with RESTful APIs
3. **Service (`crates/service`)**: Pure business logic
4. **Data (`crates/database`)**: SQLx-based persistence

### Key Principle: "Don't Touch the Body"

The router is a smart pipe, not a processor. It handles authentication and routing but streams request/response bodies with zero latency.

## Development

```bash
# Run all tests
cargo test --all-features

# Format check
cargo fmt --all -- --check

# Lint
cargo clippy --all-targets --all-features
```

## License

MIT
