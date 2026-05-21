---
name: burncloud-cli
description: Burncloud CLI command reference. Use when managing channels, tokens, users, pricing, logs, groups, protocols, currency, or monitoring the burncloud LLM gateway via CLI commands.
allowed-tools:
  - Bash
  - Read
  - Write
  - Edit
argument-hint: "<command> [args]"
---

# Burncloud CLI Reference

All commands run via `cargo run -- <command>` in the project root, or `burncloud <command>` if installed.

## channel — Manage upstream provider channels

### channel list
List all channels.
```bash
burncloud channel list
```

### channel show `<id>`
Show detailed info for a channel.
```bash
burncloud channel show 1
```

### channel add
Add a new upstream channel.
```bash
burncloud channel add \
  -t anthropic \
  -k "sk-ant-..." \
  -m "claude-sonnet-4-6,claude-opus-4-6" \
  -u "https://api.anthropic.com" \
  -n "Anthropic" \
  --pricing-region cn \
  --rpm-cap 60 \
  --tpm-cap 100000
```
| Flag | Required | Description |
|------|----------|-------------|
| `-t, --type` | Yes | Provider type: `openai`, `azure`, `anthropic`, `gemini`, `aws`, `vertexai`, `deepseek` |
| `-k, --key` | Yes | API key for the channel |
| `-m, --models` | No | Comma-separated model list (uses defaults if omitted) |
| `-u, --url` | No | Custom base URL |
| `-n, --name` | No | Channel name (uses default if omitted) |
| `--pricing-region` | No | Pricing region: `cn`, `international`, or omit for universal |
| `--rpm-cap` | No | L2 Shaper RPM cap (omit for fail-open) |
| `--tpm-cap` | No | L2 Shaper TPM cap (omit for fail-open) |
| `--reservation-green/yellow/red` | No | L2 Shaper reservation shares (0.0-1.0) |

### channel update `<id>`
Update channel config. Same flags as `channel add`, all optional.
```bash
burncloud channel update 1 --rpm-cap 120 --weight 200
```

### channel delete `<id>`
Delete a channel. Use `-y` to skip confirmation.
```bash
burncloud channel delete 1 -y
```

---

## token — Manage API tokens

### token list
List all tokens. Filter by `--user-id`.
```bash
burncloud token list
burncloud token list --user-id <uuid>
```

### token create
Create a new token for a user.
```bash
burncloud token create --name "my-token" --user-id <uuid> --unlimited
```
| Flag | Required | Description |
|------|----------|-------------|
| `--name` | No | Token name |
| `--user-id` | Yes | Owner user UUID |
| `--quota` | No | Spending quota (omit for unlimited) |
| `--unlimited` | No | Set unlimited quota |
| `--expired` | No | Expiration timestamp (-1 = never) |

### token update `<key>`
Update token properties.
```bash
burncloud token update sk-xxx --name "new-name" --status 1
```
| Flag | Description |
|------|-------------|
| `--name` | New token name |
| `--quota` | New remaining quota |
| `--status` | New status: `1`=active, `0`=disabled |

### token delete `<key>`
Delete a token. Use `-y` to skip confirmation.
```bash
burncloud token delete sk-xxx -y
```

---

## user — Manage users

### user list
List all users.
```bash
burncloud user list
```

### user register
Register a new user.
```bash
burncloud user register --username "alice" --password "secret" --email "alice@example.com"
```
| Flag | Required | Description |
|------|----------|-------------|
| `--username` | Yes | Unique username |
| `--password` | Yes | Login password |
| `--email` | No | Email address |

### user login
Authenticate a user.
```bash
burncloud user login --username "alice" --password "secret"
```

### user topup
Add balance to a user's account.
```bash
burncloud user topup --user-id <uuid> --amount 50.00 --currency USD
```
| Flag | Required | Description |
|------|----------|-------------|
| `--user-id` | Yes | User UUID |
| `--amount` | Yes | Amount (in dollars) |
| `--currency` | Yes | `USD` or `CNY` |

### user recharges
View recharge history.
```bash
burncloud user recharges --user-id <uuid>
```

### user check-username
Check if a username is available.
```bash
burncloud user check-username alice
```

---

## price — Manage model pricing

### price list
List all prices. Filter by `--currency` or `--region`.
```bash
burncloud price list
burncloud price list --currency USD --region cn
```

### price get `<model>`
Get price for a model. Filter by `--currency` or `--region`. Use `-v` for tiered pricing.
```bash
burncloud price get claude-sonnet-4-6
burncloud price get claude-sonnet-4-6 --currency USD --region cn -v
```

### price show `<model>`
Show detailed pricing across all currencies.
```bash
burncloud price show claude-sonnet-4-6
```

### price set `<model>`
Set or update a model's price.
```bash
burncloud price set claude-sonnet-4-6 --input 3.0 --output 15.0 --currency USD --region cn
```
| Flag | Required | Description |
|------|----------|-------------|
| `--input` | Yes | Input price per 1M tokens |
| `--output` | Yes | Output price per 1M tokens |
| `--currency` | No | Currency (default: USD) |
| `--region` | No | Region (omit for universal) |
| `--cache-read` | No | Cache read price per 1M tokens |
| `--cache-creation` | No | Cache creation price per 1M tokens |
| `--batch-input/output` | No | Batch pricing |
| `--alias` | No | Alias to another model's pricing |

### price delete `<model>`
Delete pricing for a model. Filter by `--region`.
```bash
burncloud price delete claude-sonnet-4-6
```

### price import `<file>`
Import prices from JSON.
```bash
burncloud price import prices.json
```

### price export `<file>`
Export prices to JSON or CSV.
```bash
burncloud price export prices.json
burncloud price export prices.csv --format csv
```

### price validate `<file>`
Validate a pricing file without applying.
```bash
burncloud price validate prices.json
```

### price sync
Sync prices from remote catalog.
```bash
burncloud price sync
```

### price sync-status
Show pricing sync status and statistics.
```bash
burncloud price sync-status
```

---

## tiered — Manage tiered pricing

### tiered list-tiers `<model>`
List pricing tiers for a model. Filter by `--region`.
```bash
burncloud tiered list-tiers claude-sonnet-4-6
```

### tiered add-tier
Add a pricing tier.
```bash
burncloud tiered add-tier --model claude-sonnet-4-6 --min-tokens 0 --max-tokens 1000000 --input 3.0 --output 15.0
```

### tiered import-tiered `<file>`
Import tiered pricing from JSON.
```bash
burncloud tiered import-tiered tiers.json
```

### tiered delete-tiers `<model>`
Delete all tiers for a model.
```bash
burncloud tiered delete-tiers claude-sonnet-4-6
```

### tiered check-tiered `<model>`
Check if a model has tiered pricing.
```bash
burncloud tiered check-tiered claude-sonnet-4-6
```

---

## protocol — Manage protocol configs

### protocol list
List all protocol configs.
```bash
burncloud protocol list
```

### protocol add
Add a protocol config.
```bash
burncloud protocol add --channel-type 1 --api-version "2023-06-01" --default --chat-endpoint "/v1/messages"
```
| Flag | Required | Description |
|------|----------|-------------|
| `--channel-type` | Yes | Type ID: 0=OpenAI, 1=Anthropic, 2=Azure, 3=AWS, 4=Gemini, 5=VertexAI, 6=DeepSeek, 7=Moonshot |
| `--api-version` | Yes | API version string |
| `--default` | No | Set as default for this channel type |
| `--chat-endpoint` | No | Chat endpoint template |
| `--embed-endpoint` | No | Embedding endpoint template |
| `--models-endpoint` | No | Models listing endpoint |
| `--request-mapping` | No | JSON request mapping config |
| `--response-mapping` | No | JSON response mapping config |
| `--detection-rules` | No | JSON detection rules |

### protocol show `<id>`
Show protocol config details.
```bash
burncloud protocol show 1
```

### protocol delete `<id>`
Delete a protocol config.
```bash
burncloud protocol delete 1
```

### protocol test
Test a protocol config against a channel.
```bash
burncloud protocol test --channel-id 1 --model claude-sonnet-4-6
```

---

## currency — Manage exchange rates

### currency list-rates
List all exchange rates.
```bash
burncloud currency list-rates
```

### currency set-rate
Set a custom exchange rate.
```bash
burncloud currency set-rate --from USD --to CNY --rate 7.25
```

### currency refresh
Refresh rates from external API.
```bash
burncloud currency refresh
```

### currency convert
Convert amount between currencies.
```bash
burncloud currency convert --from USD --to CNY 10
```

---

## group — Manage router groups

### group list
List all groups.
```bash
burncloud group list
```

### group show `<id>`
Show group details.
```bash
burncloud group show 1
```

### group create
Create a new group.
```bash
burncloud group create --name "Premium" --members "1,2"
```
| Flag | Required | Description |
|------|----------|-------------|
| `--name` | Yes | Group name |
| `--members` | No | Comma-separated upstream IDs to add as members |

### group delete `<id>`
Delete a group.
```bash
burncloud group delete 1
```

### group members `<id>`
Manage group members. Use `--set` to update members.
```bash
burncloud group members 1
burncloud group members 1 --set "1:10,2:20"
```

---

## log — View request logs and usage

### log list
List recent request logs.
```bash
burncloud log list --limit 20 --model claude-sonnet-4-6 --format json
```
| Flag | Description |
|------|-------------|
| `--user-id` | Filter by user UUID |
| `--channel-id` | Filter by channel ID |
| `--model` | Filter by model name |
| `--limit` | Max results (default: 100) |
| `--offset` | Pagination offset (default: 0) |
| `--format` | `table` (default) or `json` |

### log usage
Show aggregated usage statistics. One of `--user-id` or `--token` is required.
```bash
burncloud log usage --user-id <uuid> --period month
burncloud log usage --token sk-xxx --period week
```
| Flag | Required | Description |
|------|----------|-------------|
| `--user-id` | No* | User UUID |
| `--token` | No* | API token key (e.g. `sk-xxx`) |
| `--period` | No | `day`, `week`, `month` (default: `month`) |
| `--format` | No | `table` (default) or `json` |

*One of `--user-id` or `--token` is required.

---

## monitor — System health

### monitor status
Show system status and metrics.
```bash
burncloud monitor status
burncloud monitor status --format json
```

---

## update — Check for updates

```bash
burncloud update              # Check and apply updates
burncloud update --check-only # Check only, don't update
```

---

## install — Install third-party AI software

```bash
burncloud install --list              # List available software
burncloud install openclaw            # Install specific software
burncloud install --status            # Check installation status
burncloud install --auto-deps openclaw # Auto-install dependencies
```

---

## bundle — Manage offline bundles

### bundle create `<software>`
Create an offline installation bundle.
```bash
burncloud bundle create openclaw -o ./bundles
```

### bundle verify `<file>`
Verify bundle integrity.
```bash
burncloud bundle verify bundle.tar.gz
```

---

## Common Workflows

### Quick start: add channel + create user token
```bash
# 1. Add upstream channel
burncloud channel add -t anthropic -k "sk-ant-..." -u "https://api.anthropic.com" -n "Anthropic" --rpm-cap 60

# 2. Register user
burncloud user register --username "alice" --password "secret"

# 3. Get user UUID from list
burncloud user list

# 4. Create API token
burncloud token create --name "alice-key" --user-id <uuid> --unlimited

# 5. Test API
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: <token-key>" \
  -H "anthropic-version: 2023-06-01" \
  -d '{"model":"claude-sonnet-4-6","max_tokens":100,"messages":[{"role":"user","content":"Hello"}]}'
```

### Check usage and costs
```bash
burncloud log usage --user-id <uuid> --period month
burncloud log usage --token sk-xxx --period month
burncloud log list --limit 10 --model claude-sonnet-4-6
```

### Manage pricing
```bash
burncloud price set claude-sonnet-4-6 --input 3.0 --output 15.0
burncloud price import prices.json
burncloud price sync-status
```

### Currency and billing
```bash
burncloud currency convert --from USD --to CNY 10
burncloud user topup --user-id <uuid> --amount 50.00 --currency USD
```