# BurnCloud API Documentation

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

### Standard Error Codes

| HTTP Status | Code | Type | Description |
|-------------|------|------|-------------|
| 401 | `invalid_token` | `invalid_request_error` | Invalid or missing authentication token |
| 401 | `token_expired` | `invalid_request_error` | Token has expired |
| 402 | `insufficient_quota` | `insufficient_quota_error` | Insufficient quota for this request |
| 403 | `permission_denied` | `permission_error` | Permission denied for this resource |
| 404 | `not_found` | `not_found_error` | Resource not found |
| 429 | `rate_limit_exceeded` | `rate_limit_error` | Too many requests |
| 500 | `server_error` | `server_error` | Internal server error |
| 503 | `service_unavailable` | `server_error` | Service temporarily unavailable |

## Token Expiry

Tokens can have an expiry time set. When a token expires:

1. Request returns `401 Unauthorized`
2. Error code is `token_expired`
3. Response body:
```json
{
  "error": {
    "message": "Token has expired",
    "type": "invalid_request_error",
    "code": "token_expired"
  }
}
```

### Token Expiry Semantics

- `expired_time = -1`: Token never expires (default)
- `expired_time > 0`: Unix timestamp of expiration time

## Quota Management

### Quota Check

When making a request, the system checks if the token has sufficient quota:

1. If `quota_limit >= 0` and `used_quota >= quota_limit`, request is rejected
2. Response returns `402 Payment Required`
3. Error code is `insufficient_quota`

```json
{
  "error": {
    "message": "Insufficient quota",
    "type": "insufficient_quota_error",
    "code": "insufficient_quota"
  }
}
```

### Quota Deduction

After each successful request:

1. Token usage is calculated from prompt + completion tokens
2. Quota is deducted atomically from both user and token
3. Usage is tracked in `router_logs` table

## Pricing

### CLI Commands

List all model prices:
```bash
burncloud price list
```

Set price for a model:
```bash
burncloud price set gpt-4 --input 30.0 --output 60.0
```

Get price for a specific model:
```bash
burncloud price get gpt-4
```

Delete price:
```bash
burncloud price delete gpt-4
```

### Pricing Format

Prices are per 1 million tokens:

- `input_price`: Cost per 1M prompt tokens
- `output_price`: Cost per 1M completion tokens

Example: gpt-4 with input=$30/1M, output=$60/1M

For 100 prompt tokens + 200 completion tokens:
```
cost = (100/1,000,000 * 30) + (200/1,000,000 * 60)
     = 0.003 + 0.012
     = $0.015
```

### Model Aliases

Models can be aliased to other models for pricing:

```bash
burncloud price set gpt-4-turbo --alias gpt-4
```

This means `gpt-4-turbo` will use `gpt-4` pricing.

## Streaming Token Statistics

When using streaming requests with `stream: true`, token statistics are parsed from the response:

### OpenAI

Add `stream_options` to get usage stats:
```json
{
  "model": "gpt-4",
  "messages": [...],
  "stream": true,
  "stream_options": { "include_usage": true }
}
```

### Anthropic

Token counts are in `message_start` and `message_delta` events:
- `message_start.usage.input_tokens`
- `message_delta.usage.output_tokens`

### Gemini

Token counts are in `usageMetadata`:
- `usageMetadata.promptTokenCount`
- `usageMetadata.candidatesTokenCount`
