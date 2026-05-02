# Getting Started — BurnCloud Self-Hosted Gateway

## Prerequisites

- Rust 1.80+ (for building from source)
- SQLite 3.35+ or PostgreSQL 14+ (SQLite is the default, zero-config option)
- An upstream API key (e.g. OpenAI, Anthropic, Google Gemini)

## Quick Start

### 1. Build & Run

```bash
cargo build --release
JWT_SECRET=your-secure-random-secret ./target/release/burncloud
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

curl -s -X POST http://localhost:3000/console/api/channel \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "my-openai-channel",
    "type": 1,
    "key": "sk-your-openai-key",
    "base_url": "https://api.openai.com",
    "models": "gpt-4o-mini",
    "group": "default",
    "weight": 1,
    "priority": 0
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
| `PORT` | `3000` | Server listening port. |
| `HOST` | `0.0.0.0` | Server listening host. |
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

## Docker Deployment

### Quick Start with Docker Compose

The project includes a `docker-compose.yml` that sets up BurnCloud with PostgreSQL.

```bash
# 1. Set JWT_SECRET (required)
echo "JWT_SECRET=$(openssl rand -hex 32)" > .env

# 2. Build and start
docker compose up -d

# 3. Verify
curl http://localhost:8080/health
```

### Docker Compose Configuration

The `docker-compose.yml` defines two services:

| Service | Image | Port | Notes |
|---------|-------|------|-------|
| `burncloud` | Built from `Dockerfile` | 8080 | Multi-stage Rust build, health check on `/health` |
| `postgres` | `postgres:16-alpine` | 5432 (internal) | Data persisted in `pgdata` volume |

Key environment variables in the compose file:

- `HOST=0.0.0.0` — Required in containers to accept external connections
- `PORT=8080` — Service port (mapped to host port 8080)
- `JWT_SECRET` — **Must be set** in `.env` file before starting
- `BURNCLOUD_DATABASE_URL` — PostgreSQL connection string pointing to the `postgres` service

### Standalone Docker

To run BurnCloud alone with the default SQLite database:

```bash
docker build -t burncloud .

docker run -d \
  -p 8080:8080 \
  -e HOST=0.0.0.0 \
  -e PORT=8080 \
  -e JWT_SECRET=$(openssl rand -hex 32) \
  --name burncloud \
  burncloud
```

### Customizing the Deployment

To use a different PostgreSQL password or port, modify `docker-compose.yml` or override via environment:

```bash
# Override PostgreSQL credentials
POSTGRES_PASSWORD=mysecurepass docker compose up -d
```

To change the BurnCloud port mapping:

```yaml
# In docker-compose.yml, change the ports line:
ports:
  - "3000:8080"   # Map host port 3000 to container port 8080
```

## Troubleshooting

### "Invalid token" on API calls

- Verify you're using the JWT token from registration/login, not your upstream API key.
- Tokens expire based on `JWT_SECRET` — if you restart with a different secret, all existing tokens become invalid.
- Ensure the `Authorization: Bearer <token>` header is set correctly (no extra spaces, `Bearer` capitalized).

### Server starts but API returns 401

- Check that the `JWT_SECRET` environment variable is set and consistent across restarts.
- If using Docker, confirm the `.env` file exists and `JWT_SECRET` is set before `docker compose up`.

### "No available channel" error on `/v1/chat/completions`

- You must add at least one channel (Step 3) before making LLM requests.
- Verify the channel's `models` field includes the model you're requesting.
- Check the channel status is `1` (enabled) in the console.

### Database locked (SQLite)

- SQLite does not support concurrent writes well. If you see "database is locked" errors:
  - Use a single process (no concurrent `burncloud` instances pointing to the same `.db` file).
  - Or switch to PostgreSQL by setting `BURNCLOUD_DATABASE_URL`.

### Port already in use

- Change the port with `PORT=8080 ./target/release/burncloud`.
- If using Docker, modify the port mapping in `docker-compose.yml`.

### Dashboard shows "Not authenticated"

- The dashboard requires login. Register a user first (Step 2).
- If the dashboard page loads but data shows errors, your JWT token may have expired — log in again.

## Next Steps

- Add multiple channels for failover and load balancing
- Configure channel weights and priorities
- Set up rate limits and circuit breakers
- Review the full API documentation in `docs/api/`
