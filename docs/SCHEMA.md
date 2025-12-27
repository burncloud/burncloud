# BurnCloud Database Schema Reference

> **Status:** Draft v1.0
> **Last Updated:** 2025-12-26
> **Database Engine:** SQLite (Primary) / PostgreSQL (Optional)

This document serves as the authoritative reference for the BurnCloud database schema. It includes both **implemented** tables and **planned** tables (marked as `[Planned]`) based on the architectural blueprint.

---

## 1. Core Identity & RBAC (`database-user`)

Manages users, roles, permissions, and basic wallet balance.

### `users`
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `id` | TEXT | PK | UUID v4 |
| `username` | TEXT | UNIQUE, NOT NULL | Login name |
| `email` | TEXT | UNIQUE | Optional email |
| `password_hash` | TEXT | | bcrypt hash |
| `github_id` | TEXT | | OAuth ID |
| `status` | INTEGER | DEFAULT 1 | 1: Active, 0: Disabled |
| `balance` | REAL | DEFAULT 0.0 | Account balance (Fiat/USDT equivalent) |
| `created_at` | TEXT | DEFAULT NOW | Creation timestamp |

### `roles`
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `id` | TEXT | PK | e.g., 'role-admin' |
| `name` | TEXT | UNIQUE, NOT NULL | Display name |
| `description` | TEXT | | |

### `user_roles`
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `user_id` | TEXT | PK, FK | -> users.id |
| `role_id` | TEXT | PK, FK | -> roles.id |

### `recharges` (Finance)
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `id` | INTEGER | PK, AUTOINC | Transaction ID |
| `user_id` | TEXT | FK, NOT NULL | -> users.id |
| `amount` | REAL | NOT NULL | Positive value |
| `description` | TEXT | | e.g., "Alipay Top-up" |
| `created_at` | TEXT | DEFAULT NOW | |

---

## 2. Router & Gateway (`database-router`)

Manages LLM routing, upstream providers, and access tokens.

### `router_upstreams` (Suppliers)
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `id` | TEXT | PK | Unique upstream ID |
| `name` | TEXT | NOT NULL | Display name |
| `base_url` | TEXT | NOT NULL | API Endpoint |
| `api_key` | TEXT | NOT NULL | **Sensitive** (Should be encrypted) |
| `match_path` | TEXT | NOT NULL | e.g., `/v1/chat/completions` |
| `auth_type` | TEXT | NOT NULL | `Bearer`, `XApiKey`, `AwsSigV4` |
| `priority` | INTEGER | DEFAULT 0 | Routing priority (Higher = First) |
| `protocol` | TEXT | DEFAULT 'openai' | `openai`, `gemini`, `claude` |

### `router_tokens` (Access Keys)
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `token` | TEXT | PK | The `sk-...` key used by clients |
| `user_id` | TEXT | FK, NOT NULL | -> users.id |
| `status` | TEXT | NOT NULL | `active`, `disabled` |
| `quota_limit` | INTEGER | DEFAULT -1 | Max usage (-1 = Unlimited) |
| `used_quota` | INTEGER | DEFAULT 0 | Accumulated usage |
| `name` | TEXT | [Planned] | Token alias (e.g. "CI/CD") |
| `expired_at` | TEXT | [Planned] | Expiration timestamp |
| `allowed_models` | TEXT | [Planned] | JSON Array of model IDs |
| `ip_whitelist` | TEXT | [Planned] | JSON Array of IPs/CIDRs |

### `router_groups`
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `id` | TEXT | PK | Group ID |
| `name` | TEXT | NOT NULL | Group name |
| `strategy` | TEXT | DEFAULT 'round_robin' | Routing strategy |
| `match_path` | TEXT | NOT NULL | |

### `router_group_members`
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `group_id` | TEXT | PK, FK | -> router_groups.id |
| `upstream_id` | TEXT | PK, FK | -> router_upstreams.id |
| `weight` | INTEGER | DEFAULT 1 | Load balancing weight |

### `router_logs` (Telemetry)
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `id` | INTEGER | PK, AUTOINC | |
| `request_id` | TEXT | NOT NULL | Internal Trace ID |
| `user_id` | TEXT | | Who made the request |
| `path` | TEXT | NOT NULL | Request path |
| `upstream_id` | TEXT | | Which provider handled it |
| `status_code` | INTEGER | NOT NULL | HTTP Status |
| `latency_ms` | INTEGER | NOT NULL | Total latency |
| `prompt_tokens` | INTEGER | DEFAULT 0 | |
| `completion_tokens` | INTEGER | DEFAULT 0 | |
| `created_at` | TEXT | DEFAULT NOW | |
| `trace_id` | TEXT | [Planned] | Global Trace ID (OpenTelemetry) |
| `upstream_req_id` | TEXT | [Planned] | Provider's Request ID (e.g. `x-amzn-requestid`) |
| `direction` | TEXT | [Planned] | `INBOUND` or `OUTBOUND` |
| `cost_estimated` | REAL | [Planned] | Estimated cost in USD |

---

## 3. BurnCloud Connect (`database-connect`) [Planned]

Stores configuration for the "Mining Pool" model. To be implemented in a new crate.

### `connect_mining_nodes` (Supply Side)
*Local resources registered as miners.*
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `node_id` | TEXT | PK | Unique Miner ID |
| `upstream_id` | TEXT | FK, UNIQUE | Linked local upstream (AWS/Azure) |
| `pool_url` | TEXT | NOT NULL | Connected Mining Pool URL |
| `status` | TEXT | NOT NULL | `active`, `paused`, `banned` |
| `total_earnings` | REAL | DEFAULT 0.0 | Total earned from this node |
| `trust_score` | INTEGER | DEFAULT 100 | Local view of trust score |

### `connect_pools` (Demand Side)
*External pools we are sourcing compute from.*
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `id` | TEXT | PK | Pool ID |
| `name` | TEXT | NOT NULL | Pool Display Name |
| `base_url` | TEXT | NOT NULL | Pool API Endpoint |
| `api_token` | TEXT | NOT NULL | Auth Token for the pool |
| `balance` | REAL | DEFAULT 0.0 | Remaining credits in this pool |
| `is_active` | BOOLEAN | DEFAULT 1 | |

---

## 4. Risk Radar (`database-risk`) [Planned]

Persistent storage for threat detection and audit logs.

### `risk_events`
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `id` | INTEGER | PK, AUTOINC | |
| `event_type` | TEXT | NOT NULL | `sql_injection`, `prompt_injection`, `pii_leak` |
| `source` | TEXT | | Source IP or User ID |
| `target` | TEXT | | Upstream ID or URL |
| `severity` | TEXT | NOT NULL | `high`, `medium`, `low` |
| `action` | TEXT | NOT NULL | `blocked`, `sanitized`, `logged` |
| `payload_snapshot` | TEXT | | Redacted snippet of the malicious payload |
| `created_at` | TEXT | DEFAULT NOW | |

### `risk_rules`
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `id` | INTEGER | PK, AUTOINC | |
| `rule_type` | TEXT | NOT NULL | `keyword`, `regex`, `ip_block` |
| `content` | TEXT | NOT NULL | The rule content |
| `is_enabled` | BOOLEAN | DEFAULT 1 | |

---

## 5. System Settings (`database-setting`)

Simple Key-Value store for global configs.

### `settings`
| Column | Type | Attributes | Description |
| :--- | :--- | :--- | :--- |
| `key` | TEXT | PK | Config Key |
| `value` | TEXT | | Config Value (JSON string) |
