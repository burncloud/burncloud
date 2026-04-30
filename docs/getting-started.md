# Getting Started — BurnCloud Self-Hosted Gateway

## Prerequisites

- Rust 1.80+ (for building from source)
- SQLite 3.35+ or PostgreSQL 14+ (SQLite is the default, zero-config option)
- An upstream API key (e.g. OpenAI, Anthropic, Google Gemini)

## Quick Start

### 1. Build & Run

```bash
cargo build --release
JWT_SECRET=your-secure-random-secret ./target/release/burncloud-server
```

> **Security Warning:** The default `JWT_SECRET` is hardcoded for development only.
> Always set a strong, unique `JWT_SECRET` in production. Example:
> ```bash
> export JWT_SECRET=$(openssl rand -base64 32)
> ```

The server starts on `http://localhost:3000` by default.

### 2. Register the First User (Auto-Admin)

The first user to register automatically receives the **admin** role. All subsequent users receive the standard **user** role.

```bash
curl -s -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "YourSecurePassword!", "email": "admin@example.com"}'
```

Response:
```json
{
  "success": true,
  "data": {
    "id": "uuid...",
    "username": "admin",
    "roles": ["admin"],
    "token": "eyJ0eXAi..."
  }
}
```

Save the `token` — you'll need it for all authenticated requests.

### 3. Add an Upstream Channel

Add your upstream API provider as a channel:

```bash
TOKEN="<your-token-from-step-2>"

curl -s -X POST http://localhost:3000/console/api/channels \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "my-openai-channel",
    "base_url": "https://api.openai.com",
    "api_key": "sk-your-openai-key",
    "protocol": "openai",
    "models": ["gpt-4o-mini"],
    "status": 1
  }'
```

### 4. Make Your First API Call

Use the BurnCloud gateway just like you'd use OpenAI directly — just change the `base_url`:

```bash
curl -s -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 10
  }'
```

### 5. Check Your Billing Summary

View per-model usage and costs for your account:

```bash
curl -s http://localhost:3000/api/billing/summary \
  -H "Authorization: Bearer $TOKEN"
```

Response:
```json
{
  "success": true,
  "data": {
    "period_start": null,
    "period_end": null,
    "pre_migration_requests": 0,
    "models": [
      {
        "model": "gpt-4o-mini",
        "requests": 1,
        "prompt_tokens": 12,
        "completion_tokens": 3,
        "cost_usd": 0.0000015
      }
    ],
    "total_cost_usd": 0.0000015
  }
}
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `JWT_SECRET` | `burncloud-default-secret-change-in-production` | **Must be changed in production.** Secret key for signing JWT tokens. |
| `BURNCLOUD_INTERNAL_SECRET` | *(none)* | Shared secret for internal API endpoints. Optional. |
| `DATABASE_URL` | `sqlite:burncloud.db` | Database connection string. |
| `RUST_LOG` | `info` | Log level (`debug`, `info`, `warn`, `error`). |

## Using with OpenAI SDK

Point any OpenAI-compatible SDK at your BurnCloud gateway:

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:3000/v1",
    api_key="<your-jwt-token>"
)

response = client.chat.completions.create(
    model="gpt-4o-mini",
    messages=[{"role": "user", "content": "Hello!"}]
)
```

## Dashboard

If built with the `liveview` feature, the web dashboard is available at `http://localhost:3000/`.

## Next Steps

- Add multiple channels for failover and load balancing
- Configure channel weights and priorities
- Set up rate limits and circuit breakers
- Review the full API documentation in `docs/api/`
