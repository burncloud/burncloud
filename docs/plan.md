# BurnCloud 项目规划文档

---

## 一、智能路由系统演进规划

> 目标：打造最强全智能化企业级 AI 大模型路由系统

### 1.1 当前问题分析

#### new-api 存在的问题

多个渠道添加后，没有很好地把没有额度的渠道置后，还是不停地尝试第一个渠道。

**根本原因**：
1. 熔断器不区分失败类型，所有失败被同等对待
2. 静态优先级，无法根据实时状态动态调整
3. 缺乏渠道配额感知能力

#### 复杂性：两级状态管理

渠道配额耗尽可能只是某个模型，并不是整个渠道所有模型，需要区分处理。

```
┌────────────────────────────────────────────────────────────────┐
│                         Channel (渠道)                         │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  API Key: sk-xxx...                                      │  │
│  │  Base URL: https://api.openai.com                        │  │
│  │                                                          │  │
│  │  渠道级状态:                                              │  │
│  │  - 认证状态: ✅ OK                                        │  │
│  │  - 账户余额: ⚠️ 低 ($2.50)                                │  │
│  │  - 整体限流: ✅ 无                                        │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                │
│  支持的模型 (abilities):                                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │   gpt-4     │  │  gpt-3.5    │  │  gpt-4o     │            │
│  │ 状态: ✅ OK  │  │ 状态: ⚠️ 限流│  │ 状态: ✅ OK  │            │
│  │ 配额: 正常   │  │ 限流: 60秒后 │  │ 配额: 正常   │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### 1.2 错误类型与影响范围

| 错误码 | 错误类型 | 影响范围 | 处理策略 |
|--------|----------|----------|----------|
| **401** | 认证失败 | 🔴 渠道级 | 禁用整个渠道 |
| **402** | 余额不足 | 🔴 渠道级 | 禁用整个渠道，通知管理员 |
| **429 (account)** | 账户级限流 | 🔴 渠道级 | 冷却后重试 |
| **429 (model)** | 模型级限流 | 🟡 模型级 | 只禁用该模型 |
| **404** | 模型不存在 | 🟡 模型级 | 移除该模型能力 |
| **500/502/503** | 服务端错误 | 🟢 临时 | 正常熔断重试 |
| **超时** | 网络问题 | 🟢 临时 | 正常熔断重试 |

### 1.3 数据结构设计

#### 渠道状态追踪系统

```rust
/// 渠道状态（渠道级别）
pub struct ChannelState {
    pub channel_id: i32,

    // 渠道级状态
    pub auth_ok: bool,                    // 认证是否正常
    pub balance_status: BalanceStatus,    // 余额状态
    pub account_rate_limit_until: Option<Instant>, // 账户级限流解除时间

    // 模型级状态（Map<model_name, ModelState>）
    pub models: DashMap<String, ModelState>,
}

/// 模型状态（模型级别）
pub struct ModelState {
    pub model: String,
    pub channel_id: i32,

    // 状态
    pub status: ModelStatus,
    pub rate_limit_until: Option<Instant>,  // 模型级限流解除时间
    pub last_error: Option<String>,
    pub last_error_time: Option<Instant>,

    // 统计
    pub success_count: u32,
    pub failure_count: u32,
    pub avg_latency_ms: u32,
}

pub enum BalanceStatus {
    Ok,           // 正常
    Low,          // 低余额警告
    Exhausted,    // 耗尽
    Unknown,      // 未知（无法查询）
}

pub enum ModelStatus {
    Available,           // 可用
    RateLimited,         // 被限流
    QuotaExhausted,      // 配额耗尽
    ModelNotFound,       // 模型不存在
    TemporarilyDown,     // 临时故障
}
```

#### 状态追踪器架构

```
         ┌──────────────────────────────────────────┐
         │            ChannelStateTracker            │
         ├──────────────────────────────────────────┤
         │                                          │
         │  channel_states: DashMap<channel_id,     │
         │                   ChannelState>          │
         │                                          │
         │  ┌─────────────────────────────────────┐ │
         │  │ ChannelState {                      │ │
         │  │   auth_ok: bool,                    │ │
         │  │   balance: BalanceStatus,           │ │
         │  │   account_rate_limit: Option<Time>, │ │
         │  │   models: DashMap<model,            │ │
         │  │            ModelState>              │ │
         │  │ }                                   │ │
         │  └─────────────────────────────────────┘ │
         │                                          │
         │  Methods:                                │
         │  - is_available(channel, model) -> bool  │
         │  - record_error(channel, model, error)   │
         │  - record_success(channel, model)        │
         │  - get_available_channels(group, model)  │
         │                                          │
         └──────────────────────────────────────────┘
```

### 1.4 智能错误处理流程

```
请求失败
    │
    ├─→ 解析错误响应
    │       │
    │       ├─→ 401 Unauthorized
    │       │       └─→ channel_state.auth_ok = false
    │       │       └─→ 禁用整个渠道
    │       │
    │       ├─→ 402 Payment Required
    │       │       └─→ channel_state.balance_status = Exhausted
    │       │       └─→ 禁用整个渠道
    │       │
    │       ├─→ 429 Too Many Requests
    │       │       │
    │       │       ├─→ 检查响应体判断是账户级还是模型级
    │       │       │
    │       │       ├─→ [账户级] "Rate limit exceeded for account"
    │       │       │       └─→ channel_state.rate_limit_until = now + 60s
    │       │       │
    │       │       └─→ [模型级] "Rate limit exceeded for model 'gpt-4'"
    │       │               └─→ model_state.rate_limit_until = now + 60s
    │       │               └─→ 只禁用该渠道的该模型
    │       │
    │       ├─→ 404 Model Not Found
    │       │       └─→ model_state.status = ModelNotFound
    │       │       └─→ 从 abilities 中移除该模型
    │       │
    │       └─→ 5xx / Timeout
    │               └─→ model_state.status = TemporarilyDown
    │               └─→ 正常熔断逻辑
    │
    └─→ 返回错误给调用方（或尝试下一个渠道）
```

### 1.5 路由选择逻辑（改进版）

```rust
pub async fn select_channel(&self, group: &str, model: &str) -> Option<Channel> {
    // 1. 查询 abilities 获取候选渠道
    let candidates = self.query_abilities(group, model).await?;

    // 2. 过滤可用渠道
    let mut available = Vec::new();

    for candidate in candidates {
        let channel_state = self.get_channel_state(candidate.channel_id);

        // 渠道级检查
        if !channel_state.auth_ok {
            continue; // 认证失败，跳过
        }
        if channel_state.balance_status == BalanceStatus::Exhausted {
            continue; // 余额耗尽，跳过
        }
        if let Some(until) = channel_state.account_rate_limit_until {
            if until > Instant::now() {
                continue; // 账户级限流中，跳过
            }
        }

        // 模型级检查
        if let Some(model_state) = channel_state.models.get(model) {
            if model_state.status != ModelStatus::Available {
                continue; // 模型不可用，跳过
            }
            if let Some(until) = model_state.rate_limit_until {
                if until > Instant::now() {
                    continue; // 模型级限流中，跳过
                }
            }
        }

        available.push(candidate);
    }

    // 3. 没有可用渠道
    if available.is_empty() {
        // 触发降级逻辑
        return self.fallback_channel(group, model);
    }

    // 4. 按 priority + weight 选择
    self.weighted_select(available)
}
```

### 1.6 渠道优先级分层与降级

#### 分层架构

```
┌─────────────────────────────────────────────────────────────┐
│                    渠道优先级分层                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Tier 1: 主渠道 (priority: 100-200)                         │
│  ├── Channel A: OpenAI 官方，高配额，高成本                   │
│  └── Channel B: Azure OpenAI，高配额，高成本                  │
│                                                             │
│  Tier 2: 备选渠道 (priority: 50-99)                         │
│  ├── Channel C: OpenAI 代理，中等配额，中等成本               │
│  └── Channel D: 第三方聚合，中等配额                         │
│                                                             │
│  Tier 3: 降级渠道 (priority: 1-49)                          │
│  ├── Channel E: 本地模型，无限额，低成本，质量较低            │
│  └── Channel F: 开源模型代理，低成本                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 降级策略

1. 正常情况只用 Tier 1
2. Tier 1 全部不可用时，自动降级到 Tier 2
3. Tier 2 也不可用时，降级到 Tier 3（本地模型兜底）
4. 可配置"拒绝服务"而非降级（适用于对质量要求高的场景）

### 1.7 功能规划总表

#### 智能路由核心

| 功能 | 说明 | 优先级 |
|------|------|--------|
| **多供应商聚合** | 统一接入 OpenAI、Claude、Gemini、文心、通义、本地模型等 | ✅ 已实现 |
| **智能模型选择** | 根据请求特征自动选择最优模型（成本/延迟/能力） | P1 |
| **协议自适应** | OpenAI/Claude/Gemini/Vertex 协议自动转换 | ✅ 已实现 |
| **请求路由策略** | 轮询、权重、最小延迟、成本优先、能力匹配 | ✅ 已实现 |
| **语义路由** | 根据 prompt 内容智能路由到擅长该领域的模型 | P2 |

#### 高可用与容错

| 功能 | 说明 | 优先级 |
|------|------|--------|
| **熔断降级** | 自动检测上游故障，熔断后降级到备选模型 | ✅ 已实现（需改进） |
| **智能失败分类** | 区分认证失败、配额耗尽、限流、临时故障 | **P0** |
| **渠道状态追踪** | 渠道级 + 模型级两级状态管理 | **P0** |
| **健康检查** | 主动探测上游端点健康状态 | P1 |
| **流量调度** | 多地域部署、就近路由、灾备切换 | P2 |

#### 安全与治理

| 功能 | 说明 | 优先级 |
|------|------|--------|
| **统一认证** | API Key、OAuth、JWT、IP白名单 | ✅ 已实现 |
| **精细权限** | 按用户/团队/项目的模型访问控制 | ✅ 已实现 |
| **速率限制** | TPM/RPM/并发数多维限流 | ✅ 已实现 |
| **内容安全** | 敏感词过滤、PII脱敏、输出审核 | P1 |
| **审计追踪** | 完整请求日志、合规报告 | P1 |

#### 成本与配额管理

| 功能 | 说明 | 优先级 |
|------|------|--------|
| **配额管理** | 按用户/团队设置 Token/金额配额 | ✅ 已实现 |
| **成本追踪** | 实时统计各模型/用户消耗 | P0 |
| **预算告警** | 阈值预警、超额阻断 | P1 |
| **价格优化** | 自动选择性价比最优模型 | P1 |

#### 可观测性

| 功能 | 说明 | 优先级 |
|------|------|--------|
| **实时监控** | QPS、延迟、错误率、Token 消耗 | P0 |
| **链路追踪** | 请求全链路追踪 | P1 |
| **告警系统** | 异常检测、智能告警 | P1 |
| **数据分析** | 使用趋势、成本分析、模型效果对比 | P2 |

#### 高级智能化功能

| 功能 | 说明 | 优先级 |
|------|------|--------|
| **Prompt 优化** | 自动压缩、格式化、注入上下文 | P2 |
| **缓存层** | 语义相似请求缓存、减少重复调用 | P1 |
| **模型回退链** | 主模型失败自动降级到备选模型 | **P0** |
| **A/B 测试** | 不同模型效果对比测试 | P2 |
| **自动扩缩** | 根据负载自动调整资源 | P2 |

### 1.8 实现计划

#### Phase 1: 核心路由增强（P0）

**目标**：解决渠道选择不智能的问题

##### 1.1 渠道状态追踪系统

- [ ] 创建 `ChannelStateTracker` 模块
- [ ] 实现 `ChannelState` 和 `ModelState` 数据结构
- [ ] 实现状态持久化（可选，内存优先）

##### 1.2 智能错误分类

- [ ] 扩展 `CircuitBreaker` 支持失败类型分类
- [ ] 实现各供应商错误响应解析器
  - [ ] OpenAI 错误格式
  - [ ] Anthropic 错误格式
  - [ ] Gemini 错误格式
  - [ ] 通用 HTTP 错误
- [ ] 实现差异化处理逻辑

##### 1.3 路由选择改进

- [ ] 改进 `ModelRouter.route()` 方法
- [ ] 集成渠道状态过滤
- [ ] 实现降级渠道选择逻辑

#### Phase 2: 可观测性与监控（P1）

- [ ] 渠道健康状态 API
- [ ] 实时监控仪表板
- [ ] 告警配置系统
- [ ] 成本分析报表

#### Phase 3: 高级特性（P2）

- [ ] 语义缓存
- [ ] 智能模型推荐
- [ ] A/B 测试框架

### 1.9 关键代码位置

| 功能 | 文件位置 | 说明 |
|------|---------|------|
| Channel 数据结构 | `crates/common/src/types.rs:200` | `struct Channel` |
| Ability 数据结构 | `crates/common/src/types.rs:238` | `struct Ability` |
| 模型路由 | `crates/router/src/model_router.rs` | `ModelRouter::route()` |
| 熔断器 | `crates/router/src/circuit_breaker.rs` | 需改进 |
| 限流器 | `crates/router/src/limiter.rs` | 已实现 |
| 完整路由逻辑 | `crates/router/src/lib.rs:347` | `proxy_logic()` |
| Channel CRUD | `crates/database/crates/database-models/src/lib.rs` | `ChannelModel::*` |

### 1.10 参考资源

- OpenAI API 错误码文档: https://platform.openai.com/docs/guides/error-codes
- Anthropic API 错误处理: https://docs.anthropic.com/claude/reference/errors
- Google Gemini API 错误: https://ai.google.dev/api/errors

---

## 二、CLI Channel Management Tool (已完成 ✅)

### Context

 用户需要一个命令行工具来管理 API 渠道（Gemini, AWS, Azure, OpenAI 等），在本地运行，直接操作 SQLite 数据库，无需 HTTP
 认证。当前所有渠道管理都通过 Web API（需要认证），CLI 缺少此功能。                                                            

 Command Design

 burncloud channel <subcommand> [options]

 Subcommands:
   add       添加新渠道
   list      列出所有渠道
   delete    删除渠道
   show      显示渠道详情

 Examples

 # 添加 Gemini 渠道
 burncloud channel add -t gemini -k "AIza..." -m "gemini-1.5-pro,gemini-1.5-flash"

 # 添加 Azure 渠道
 burncloud channel add -t azure -k "azure-key" -u "https://my-resource.openai.azure.com" -m "gpt-4,gpt-35-turbo"

 # 添加 AWS 渠道
 burncloud channel add -t aws -k "AKIA...,SECRET" -m "claude-3-sonnet"

 # 列出渠道
 burncloud channel list
 burncloud channel list --format json

 # 删除渠道
 burncloud channel delete 1

 Implementation Steps

 Step 1: Update CLI Dependencies

 File: crates/cli/Cargo.toml

 Add dependencies:
 burncloud-database.workspace = true
 burncloud-database-models.workspace = true

 Step 2: Create Channel Commands Module

 File: crates/cli/src/channel.rs (NEW)

 Create module with:
 - handle_channel_command() - Route subcommands
 - cmd_channel_add() - Add channel with type parsing
 - cmd_channel_list() - List channels (table/json format)
 - cmd_channel_delete() - Delete with confirmation
 - cmd_channel_show() - Show channel details
 - parse_channel_type() - Map string to ChannelType enum
 - get_default_models() - Default models per type
 - get_default_base_url() - Default URL per type

 Step 3: Integrate with Main Command Handler

 File: crates/cli/src/commands.rs

 Add channel subcommand:
 .subcommand(
     Command::new("channel")
         .about("Manage API channels (local, no auth)")
         .subcommand_required(true)
         .subcommand(channel_add_command())
         .subcommand(channel_list_command())
         .subcommand(channel_delete_command())
         .subcommand(channel_show_command())
 )

 Handle in match block:
 Some(("channel", sub_m)) => {
     let db = Database::new().await?;
     handle_channel_command(&db, sub_m).await?;
     db.close().await?;
 }

 Step 4: Update lib.rs Exports

 File: crates/cli/src/lib.rs

 pub mod channel;
 pub use channel::*;

 Critical Files

 ┌───────────────────────────────────┬───────────────────────────────────────────┐
 │               File                │                  Purpose                  │
 ├───────────────────────────────────┼───────────────────────────────────────────┤
 │ crates/cli/src/channel.rs         │ NEW - Channel command implementations     │
 ├───────────────────────────────────┼───────────────────────────────────────────┤
 │ crates/cli/src/commands.rs        │ Add channel subcommand routing            │
 ├───────────────────────────────────┼───────────────────────────────────────────┤
 │ crates/cli/src/lib.rs             │ Export new module                         │
 ├───────────────────────────────────┼───────────────────────────────────────────┤
 │ crates/cli/Cargo.toml             │ Add database dependencies                 │
 ├───────────────────────────────────┼───────────────────────────────────────────┤
 │ crates/common/src/types.rs        │ Channel struct, ChannelType enum (exists) │
 ├───────────────────────────────────┼───────────────────────────────────────────┤
 │ crates/database-models/src/lib.rs │ ChannelModel CRUD (exists)                │
 └───────────────────────────────────┴───────────────────────────────────────────┘

 Key Reusable Functions

 From crates/database-models/src/lib.rs:
 - ChannelModel::create(db, &mut channel) -> Result<i32>
 - ChannelModel::list(db, limit, offset) -> Result<Vec<Channel>>
 - ChannelModel::get_by_id(db, id) -> Result<Option<Channel>>
 - ChannelModel::delete(db, id) -> Result<()>

 From crates/database/src/database.rs:
 - Database::new().await - Initialize DB with default path
 - db.close().await - Clean shutdown

 From crates/common/src/types.rs:
 - Channel struct with all fields
 - ChannelType enum (Gemini=24, Aws=33, Azure=3, OpenAI=1, etc.)

 Channel Type Reference

 ┌───────────┬───────┬──────────────────────────────────────────────┐
 │ Type Name │ Value │                Default Models                │
 ├───────────┼───────┼──────────────────────────────────────────────┤
 │ openai    │ 1     │ gpt-4,gpt-4-turbo,gpt-3.5-turbo              │
 ├───────────┼───────┼──────────────────────────────────────────────┤
 │ azure     │ 3     │ gpt-4,gpt-35-turbo                           │
 ├───────────┼───────┼──────────────────────────────────────────────┤
 │ anthropic │ 14    │ claude-3-opus,claude-3-sonnet,claude-3-haiku │
 ├───────────┼───────┼──────────────────────────────────────────────┤
 │ gemini    │ 24    │ gemini-1.5-pro,gemini-1.5-flash,gemini-pro   │
 ├───────────┼───────┼──────────────────────────────────────────────┤
 │ aws       │ 33    │ claude-3-sonnet,claude-3-haiku               │
 ├───────────┼───────┼──────────────────────────────────────────────┤
 │ vertexai  │ 41    │ gemini-1.5-pro                               │
 ├───────────┼───────┼──────────────────────────────────────────────┤
 │ deepseek  │ 43    │ deepseek-chat,deepseek-coder                 │
 └───────────┴───────┴──────────────────────────────────────────────┘

 Verification

 1. Build: cargo build
 2. Test add: cargo run -- channel add -t gemini -k "test-key" -m "gemini-pro"
 3. Test list: cargo run -- channel list
 4. Test delete: cargo run -- channel delete 1
 5. Verify DB: Check ~/.burncloud/data.db contains correct data

---

## 三、MVP 上线计划

### 目标

最小化产品上线，实现三大核心功能：
1. **Router直接穿透** - 请求透明转发，协议适配
2. **统计计费** - 用量统计，费用计算，配额扣除
3. **令牌分发** - Token管理，认证授权

### 当前状态

| 功能模块 | 完成度 | 状态 |
|---------|-------|------|
| Router穿透转发 | 90% | ✅ 基本可用 |
| Token分发认证 | 95% | ✅ 基本可用 |
| Channel管理 | 95% | ✅ 基本可用 |
| 模型路由 | 90% | ✅ 基本可用 |
| 统计计费 | 70% | ⚠️ 需补充 |

### MVP 任务优先级

#### P0 - 上线必需（阻塞项）

| 任务 | 问题 | 影响 | 位置 |
|------|------|------|------|
| 流式响应Token统计 | completion_tokens 固定为 0 | 无法准确计费 | `crates/router/src/lib.rs` |
| Token过期验证 | expired_time 未检查 | 过期Token仍可用 | `crates/database/*/lib.rs` |
| 计费规则配置 | 无price表/计费逻辑 | 只能统计无法计费 | 新增 `service-pricing` |

#### P1 - 建议完成（核心体验）

| 任务 | 问题 | 影响 |
|------|------|------|
| 配额扣除逻辑 | used_quota 未关联余额 | 配额形同虚设 |
| Token访问时间 | accessed_time 从未更新 | 无法追踪使用 |
| 加权负载均衡 | 仅Round-Robin | 无法按权重分流 |

### 实现阶段

#### 阶段一：流式Token统计 (P0.1)
- 解析SSE数据块中的token使用信息
- 累加并记录completion_tokens到router_logs

#### 阶段二：Token过期验证 (P0.2)
- 修改validate_token()检查expired_time
- 返回适当的错误码

#### 阶段三：计费系统 (P0.3)
- 创建prices表存储模型定价
- 实现PricingService服务层
- 请求完成后计算费用并扣除配额

#### 阶段四：配额扣除 (P1.1)
- 关联users.quota与tokens.remain_quota
- 实现原子性扣费操作

#### 阶段五：访问时间更新 (P1.2)
- Token验证时更新accessed_time

#### 阶段六：加权负载均衡 (P1.3)
- 实现WeightedBalancer
- 根据Channel权重分配请求

### 数据库变更

```sql
-- 新增 prices 表
CREATE TABLE prices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model VARCHAR(255) NOT NULL UNIQUE,
    input_price REAL NOT NULL,      -- 每千token输入价格
    output_price REAL NOT NULL,     -- 每千token输出价格
    currency VARCHAR(16) DEFAULT 'USD',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- tokens 表添加访问时间字段 (如不存在)
ALTER TABLE tokens ADD COLUMN accessed_time INTEGER;
```

### 上线检查清单

- [ ] 流式请求正确统计 completion_tokens
- [ ] 过期 Token 返回正确错误
- [ ] 计费金额计算正确
- [ ] 配额扣除原子性保证
- [ ] Token 访问时间正确更新
- [ ] 加权负载均衡按权重分配
- [ ] 单元测试覆盖核心逻辑
- [ ] 集成测试通过
- [ ] 数据库迁移脚本准备
- [ ] 配置文件模板更新
