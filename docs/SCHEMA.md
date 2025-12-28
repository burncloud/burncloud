# BurnCloud 数据库架构参考 (Database Schema Reference)

> **状态:** Draft v1.2
> **最后更新:** 2025-12-27
> **数据库引擎:** SQLite (主) / PostgreSQL (可选)

本文档是 BurnCloud 数据库架构的权威参考。它包含了基于架构蓝图 (Blueprint) 的**已实现**表和**计划中**的表 (标记为 `[Planned]`)。

---

## 1. 核心身份与权限 (`user`)
> 对应 Blueprint: **8. 用户管理 (User Management)** & **9. 财务中心 (Finance Center)**

管理用户、角色、权限以及基础钱包余额。

### `users` (用户表)
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | TEXT | PK | UUID v4 |
| `username` | TEXT | UNIQUE, NOT NULL | 登录用户名 |
| `email` | TEXT | UNIQUE | 可选邮箱 |
| `password_hash` | TEXT | | bcrypt 哈希值 |
| `github_id` | TEXT | | OAuth ID |
| `status` | INTEGER | DEFAULT 1 | 1: 激活, 0: 禁用 |
| `balance` | REAL | DEFAULT 0.0 | 账户余额 (法币/USDT 等值) |
| `rate_multiplier` | REAL | DEFAULT 1.0 | 计费倍率 (e.g. 1.2x) |
| `created_at` | TEXT | DEFAULT NOW | 创建时间 |

### `roles` (角色表)
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | TEXT | PK | e.g., 'role-admin' |
| `name` | TEXT | UNIQUE, NOT NULL | 显示名称 |
| `description` | TEXT | | 描述 |

### `user_roles` (用户角色关联)
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `user_id` | TEXT | PK, FK | -> users.id |
| `role_id` | TEXT | PK, FK | -> roles.id |

### `recharges` (充值记录)
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | INTEGER | PK, AUTOINC | 交易 ID |
| `user_id` | TEXT | FK, NOT NULL | -> users.id |
| `amount` | REAL | NOT NULL | 正数金额 |
| `description` | TEXT | | 备注, e.g., "支付宝充值" |
| `created_at` | TEXT | DEFAULT NOW | |

---

## 2. 路由与网关 (`router`)
> 对应 Blueprint: **2. 模型网络 (Model Network)** & **1. 仪表盘 (Dashboard)**

管理 LLM 路由、上游供应商和访问令牌。

### `models` (模型定义)
*标准化模型定义，独立于具体供应商。*
> 支持 Blueprint: **2. 模型网络** (统一接口)
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | TEXT | PK | 模型 ID, e.g., `gpt-4`, `claude-3-opus` |
| `name` | TEXT | NOT NULL | 显示名称 |
| `type` | TEXT | NOT NULL | `text` (文本), `vision` (视觉), `embedding` (嵌入) |
| `context_window` | INTEGER | DEFAULT 4096 | 最大上下文长度 |
| `max_tokens` | INTEGER | DEFAULT 4096 | 最大输出 Token |
| `pricing_in` | REAL | | 每 1M 输入 Token 价格 |
| `pricing_out` | REAL | | 每 1M 输出 Token 价格 |

### `channels` (供应商/上游)
> 代码对应: `crates/database/src/schema.rs` (channels)
> 对应 Blueprint: **1. 仪表盘** (上游健康矩阵)
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | INTEGER | PK, AUTOINC | 渠道 ID |
| `type` | INTEGER | DEFAULT 0 | 渠道类型 |
| `key` | TEXT | NOT NULL | **敏感信息** (应加密存储) |
| `name` | TEXT | | 显示名称 |
| `base_url` | TEXT | | API 端点地址 |
| `models` | TEXT | | 支持的模型列表 (逗号分隔 or JSON) |
| `group` | TEXT | DEFAULT 'default' | 分组 (Legacy) |
| `priority` | INTEGER | DEFAULT 0 | 路由优先级 |
| `weight` | INTEGER | DEFAULT 0 | 权重 |
| `status` | INTEGER | DEFAULT 1 | 1: 启用, 2: 禁用 |
| `health_status` | TEXT | DEFAULT 'green' | `green`, `yellow`, `red` [Blueprint] |
| `error_rate_1h` | REAL | DEFAULT 0.0 | 最近一小时错误率 [Blueprint] |
| `last_checked_at` | TEXT | | 上次健康检查时间 [Blueprint] |
| `other_info` | TEXT | | JSON: 额外配置 |

### `tokens` (访问凭证)
> 代码对应: `crates/database/src/schema.rs` (tokens)
> 对应 Blueprint: **4. 访问凭证**
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | INTEGER | PK, AUTOINC | |
| `user_id` | TEXT | FK, NOT NULL | -> users.id |
| `key` | CHAR(48) | NOT NULL, UNIQUE | `sk-...` 密钥 |
| `status` | INTEGER | DEFAULT 1 | 1: 启用, 0: 禁用 |
| `name` | VARCHAR(255) | | 别名 |
| `remain_quota` | INTEGER | DEFAULT 0 | 剩余额度 |
| `unlimited_quota` | BOOLEAN | DEFAULT 0 | 是否无限额度 |
| `used_quota` | INTEGER | DEFAULT 0 | 已用额度 |
| `created_time` | INTEGER | | 创建时间戳 |
| `expired_time` | INTEGER | DEFAULT -1 | 过期时间戳 |
| `allowed_models` | TEXT | | JSON 数组: 允许访问的模型 ID [Blueprint] |
| `ip_whitelist` | TEXT | | JSON 数组: 允许的 IP/CIDR [Blueprint] |
| `application_id` | TEXT | | 应用绑定 ID [Blueprint] |

### `abilities` (路由能力表)
> 代码对应: `crates/database/src/schema.rs` (abilities)
> 说明: NewAPI 风格的核心路由表，映射 Group + Model -> Channel
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `group` | VARCHAR(64) | PK | 分组名称 |
| `model` | VARCHAR(255) | PK | 模型名称 |
| `channel_id` | INTEGER | PK | -> channels.id |
| `enabled` | BOOLEAN | DEFAULT 1 | 是否启用 |
| `priority` | INTEGER | DEFAULT 0 | 优先级 |
| `weight` | INTEGER | DEFAULT 0 | 权重 |

### `router_groups` (路由组) [Planned/Redesign]
> 对应 Blueprint: **路由策略** (更高级的策略组)
*注意：目前代码主要使用 `abilities` 表进行路由，此表为未来更复杂的策略（如 A/B Test）预留。*

| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | TEXT | PK | 组 ID |
| `name` | TEXT | NOT NULL | 组名称 |
| `strategy` | TEXT | DEFAULT 'round_robin' | 路由策略 |
| `match_path` | TEXT | NOT NULL | 匹配路径 |

### `router_group_members` (组成员)
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `group_id` | TEXT | PK, FK | -> router_groups.id |
| `upstream_id` | TEXT | PK, FK | -> router_upstreams.id |
| `weight` | INTEGER | DEFAULT 1 | 负载均衡权重 |

### `router_logs` (遥测/日志)
> 支持 Blueprint: **7. 日志审查 (Log Review)**
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | INTEGER | PK, AUTOINC | |
| `request_id` | TEXT | NOT NULL | 内部请求追踪 ID |
| `user_id` | TEXT | | 发起请求的用户 |
| `path` | TEXT | NOT NULL | 请求路径 |
| `upstream_id` | TEXT | | 处理请求的上游 ID |
| `status_code` | INTEGER | NOT NULL | HTTP 状态码 |
| `latency_ms` | INTEGER | NOT NULL | 总耗时 (毫秒) |
| `prompt_tokens` | INTEGER | DEFAULT 0 | 提示词 Token 数 |
| `completion_tokens` | INTEGER | DEFAULT 0 | 补全 Token 数 |
| `cost_estimated` | REAL | DEFAULT 0.0 | 预估成本 (USD) |
| `created_at` | TEXT | DEFAULT NOW | |
| `trace_id` | TEXT | NOT NULL | 全局 Trace ID (OpenTelemetry) |
| `upstream_req_id` | TEXT | | 供应商 Request ID (e.g., `x-amzn-requestid`) |
| `direction` | TEXT | DEFAULT 'INBOUND' | `INBOUND` (入站/买家), `OUTBOUND` (出站/供应商) |

---

## 3. 算力互联 (`connect`) [Planned]
> 对应 Blueprint: **3. 算力互联 (BurnCloud Connect)**

存储“算力接入”模式的配置。将在新 crate 中实现。

### `connect_supply_nodes` (供应端/节点)
*作为供应商注册的本地资源。*
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `node_id` | TEXT | PK | 唯一节点 ID |
| `upstream_id` | TEXT | FK, UNIQUE | 关联的本地上游配置 (AWS/Azure) |
| `cluster_url` | TEXT | NOT NULL | 连接的算力集群 URL |
| `status` | TEXT | NOT NULL | `active` (活跃), `paused` (暂停), `banned` (封禁) |
| `total_revenue` | REAL | DEFAULT 0.0 | 该节点产生的总收益 |
| `trust_score` | INTEGER | DEFAULT 100 | 本地记录的信任分 |

### `connect_clusters` (需求端/集群)
*我们要从中获取算力的外部集群。*
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | TEXT | PK | 集群 ID |
| `name` | TEXT | NOT NULL | 集群显示名称 |
| `base_url` | TEXT | NOT NULL | 集群 API 端点 |
| `api_token` | TEXT | NOT NULL | 集群认证 Token |
| `balance` | REAL | DEFAULT 0.0 | 在该集群的剩余额度 |
| `is_active` | BOOLEAN | DEFAULT 1 | |

---

## 4. 风控雷达 (`risk`) [Planned]
> 对应 Blueprint: **6. 风控雷达 (Risk Radar)**

威胁检测与审计日志的持久化存储。

### `risk_events` (风控事件)
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | INTEGER | PK, AUTOINC | |
| `event_type` | TEXT | NOT NULL | `sql_injection`, `prompt_injection`, `pii_leak` |
| `source` | TEXT | | 来源 IP 或 User ID |
| `target` | TEXT | | 上游 ID 或 URL |
| `severity` | TEXT | NOT NULL | `high` (高), `medium` (中), `low` (低) |
| `action` | TEXT | NOT NULL | `blocked` (拦截), `sanitized` (清洗), `logged` (记录) |
| `payload_snapshot` | TEXT | | 恶意 Payload 快照 (已脱敏) |
| `created_at` | TEXT | DEFAULT NOW | |

### `risk_rules` (风控规则)
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | INTEGER | PK, AUTOINC | |
| `rule_type` | TEXT | NOT NULL | `keyword` (关键字), `regex` (正则), `ip_block` (IP黑名单) |
| `content` | TEXT | NOT NULL | 规则内容 |
| `is_enabled` | BOOLEAN | DEFAULT 1 | 是否启用 |

---

## 5. 系统设置 (`setting`)
> 对应 Blueprint: **10. 系统设置 (System Settings)**

简单的全局配置 KV 存储。

### `settings` (配置表)
| 字段 (Column) | 类型 (Type) | 属性 (Attributes) | 说明 (Description) |
| :--- | :--- | :--- | :--- |
| `key` | TEXT | PK | 配置键 |
| `value` | TEXT | | 配置值 (JSON 字符串) |
