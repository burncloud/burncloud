# BurnCloud 架构演进规划 (Architecture Evolution)

**目标**: 对标 **New-API (One API)**，构建运营级、高可用的大模型聚合平台。

当前 BurnCloud 的架构以高性能透传 Router 为核心。为了实现运营级能力（多渠道聚合、统一格式、精确计费、复杂调度），我们需要在保持核心哲学（性能至上、Rust优先）的前提下，对系统分层进行升级。

---

## 1. 系统分层架构建议 (System Layering)

建议将系统明确划分为四大核心层级，职责分离，单向依赖。

### 1.1 Gateway Layer (网关/路由层) - `crates/router`
**定位**: Data Plane (数据面) —— 系统的高速公路。
**职责**: 处理高并发流量，实现路由分发、协议适配和基础流控。

*   **当前能力**: 基础转发、AWS SigV4 签名。
*   **演进方向**:
    *   **Middleware (中间件链)**: 
        *   `AuthMiddleware`: 统一鉴权。
        *   `RateLimitMiddleware`: 基于 Redis 的精准限流。
        *   `MeteringMiddleware`: 流式计费 (Stream Metering)。
    *   **Smart Load Balancer (智能负载均衡)**:
        *   同一个模型名称 (如 `gpt-4`) 映射多个 `Upstream`。
        *   支持策略: `RoundRobin` (轮询), `Weighted` (权重), `Latency` (最小延迟)。
        *   **故障转移 (Failover)**: 上游报错自动重试下一个渠道。
    *   **Protocol Adaptors (协议适配器)**:
        *   虽然宪法强调透传，但为了支持“万物转 OpenAI”的统一体验，需引入可选的适配层。
        *   职责: 将 Google Gemini、Claude、阿里 Qwen 等非 OpenAI 格式的请求/响应，转换为 OpenAI 标准格式。
        *   *注意*: 适配器应作为可插拔组件，对原生 OpenAI 请求保持零损耗透传。

### 1.2 Control Layer (控制/管理层) - `crates/server`
**定位**: Control Plane (控制面) —— 系统的指挥中心。
**职责**: 面向管理员和前端 UI，处理配置管理、用户管理、渠道管理等低频高价值请求。

*   **当前能力**: 基础骨架。
*   **演进方向**:
    *   **Management API**: 提供 RESTful API 供前端 (`crates/client`) 调用。
        *   `/api/user`: 用户注册、登录、邀请机制。
        *   `/api/channel`: 渠道增删改查、批量导入、参数配置。
        *   `/api/log`: 调用日志查询。
    *   **System Config**: 全局系统设置（类似 New-API 的“设置”页面）。

### 1.3 Service Layer (业务逻辑层) - `crates/service`
**定位**: Business Logic (业务逻辑) —— 连接 UI、API 和数据库的桥梁。
**职责**: 封装复杂的业务规则，确保数据一致性。

*   **演进方向**:
    *   **UserService**: 处理用户额度、充值、分组管理 (Group)。
    *   **ChannelService**: 
        *   渠道自动禁用逻辑 (Monitor)。
        *   渠道优先级与权重计算。
    *   **BillingService**: 
        *   复杂的倍率计算 (Model Ratio + Group Ratio)。
        *   兑换码 (Redeem Code) 生成与核销。
    *   **LogService**: 异步处理日志入库，避免阻塞主线程。

### 1.4 Data Access Layer (数据持久层) - `crates/database`
**定位**: Persistence (持久化)。
**职责**: 数据存储与检索。

*   **演进方向**:
    *   **Advanced Queries**: 支持复杂的统计查询（如“用户月度消耗报表”）。
    *   **Caching (Redis)**: 引入 Redis 支持（可选但推荐），用于：
        *   高性能 Token 校验 (避免每次请求查 DB)。
        *   分布式限流。
        *   缓存渠道健康状态。

---

## 2. 目录结构演进 (Directory Structure)

建议在 `crates/` 下进行如下细化，以支撑上述架构：

```text
crates/
├── router/                 # [Data Plane] 高性能网关
│   ├── src/
│   │   ├── middleware/     # 鉴权、限流、计费中间件
│   │   ├── balancer/       # 负载均衡策略 (轮询/权重/故障转移)
│   │   └── adaptor/        # (新) 协议转换器 (Gemini/Claude -> OpenAI)
│   └── crates/
│       └── router-aws/     # 现有的 AWS 签名逻辑
│
├── server/                 # [Control Plane] 管理 API 服务器
│   └── src/
│       ├── api/            # 路由定义 (/api/user, /api/channel)
│       └── task/           # 后台定时任务 (渠道测速、清理日志)
│
├── service/                # [Business Logic] 业务逻辑封装
│   └── src/
│       ├── user.rs         # 用户逻辑
│       ├── channel.rs      # 渠道逻辑
│       ├── billing.rs      # 计费与倍率
│       └── monitor.rs      # 监控与健康检查
│
├── common/ (或 model)      # [Shared] 统一数据模型
│   # 定义通用的 Request/Response 结构 (如 OpenAI ChatCompletion)
│   # 供 Router 解析、Adaptor 转换和 Client 共用
│
└── database/               # [Persistence] SQLx 实现
```

## 3. 关键缺失能力与行动计划 (Gap Analysis)

要达到 New-API 的水平，目前最紧迫需要补齐的能力：

1.  **统一协议适配 (Uniform Protocol Adaptation)**
    *   *现状*: 仅透传。
    *   *目标*: 实现 `OpenAI <-> Claude` 和 `OpenAI <-> Gemini` 的双向转换层。

2.  **智能负载均衡 (Smart Load Balancing)**
    *   *现状*: 单路径匹配单一上游。
    *   *目标*: 一个模型 ID 对应一个 `ChannelGroup`，内部包含多个 `Upstream`，支持故障自动切换。

3.  **精确计费与统计 (Billing & Metering)**
    *   *现状*: 仅有基础请求计数。
    *   *目标*: 解析响应中的 `usage` 字段（或通过 tokenizer 估算流式响应），实现精确到 Token 的扣费。

4.  **Redis 集成 (Caching)**
    *   *目标*: 为了应对高并发，必须引入缓存层来存储热点数据（如 Token 有效性、渠道状态）。
