# BurnCloud 版本状态分析与 v0.4 设计文档

> **Last Updated:** 2026-07-24
> **Status:** Active Analysis

---

## 一、版本状态总览

| 版本 | 功能模块 | 状态 | 完成度 | 说明 |
|------|---------|------|--------|------|
| **v0.1** | 基础路由与 AWS SigV4 签名 | ✅ 已完成 | 100% | 路由核心、协议适配、AWS签名完整实现 |
| **v0.2** | 数据库集成、基本认证、新API核心 | ✅ 已完成 | 100% | SQLite/PostgreSQL支持、JWT认证、完整API层 |
| **v0.3** | 统一协议适配器与端对端测试 | ✅ 已完成 | 95% | OpenAI/Gemini/Claude适配器完整，E2E测试基本覆盖 |
| **v0.4** | 智能负载均衡与故障转移 | ⚠️ 进行中 | 60% | 基础熔断/健康探测完成，智能负载均衡待完善 |

---

## 二、各版本详细分析

### v0.1: 基础路由与 AWS SigV4 签名支持 ✅

**已实现模块:**

| 模块 | 文件路径 | 功能描述 |
|------|---------|---------|
| 路由核心 | `crates/router/src/lib.rs` | 请求代理流水线、协议转换 |
| AWS SigV4 | `crates/router/crates/router-aws/src/lib.rs` | 完整的 AWS Bedrock 签名实现 |
| 配置管理 | `crates/router/src/config.rs` | Upstream/Group/AuthType 配置定义 |

**验证结果:** ✅ 编译通过，签名功能测试覆盖

---

### v0.2: 数据库集成、基本认证及新API核心 ✅

**已实现模块:**

| 模块 | 文件路径 | 功能描述 |
|------|---------|---------|
| 数据库核心 | `crates/database/src/database.rs` | SQLite/PostgreSQL 统一连接 |
| 用户模块 | `crates/database/crates/user/` | 用户账户、API Key、充值 |
| 通道模块 | `crates/database/crates/channel/` | 通道配置、能力管理 |
| 计费模块 | `crates/database/crates/billing/` | 价格、订阅、分层定价 |
| 认证API | `crates/server/src/api/auth.rs` | 登录、注册、JWT验证 |
| 通道API | `crates/server/src/api/channel.rs` | 通道CRUD |
| Token API | `crates/server/src/api/token.rs` | API Key管理 |

**验证结果:** ✅ 编译通过，数据库迁移完整，API层可用

---

### v0.3: 统一协议适配器与端对端测试 ✅

**已实现模块:**

| 模块 | 文件路径 | 功能描述 |
|------|---------|---------|
| OpenAI适配器 | `crates/router/src/adaptor/gemini.rs` | OpenAI协议适配 |
| Gemini适配器 | `crates/router/src/adaptor/gemini.rs` | Gemini原生协议适配 |
| Claude适配器 | `crates/router/src/adaptor/claude.rs` | Claude原生协议适配 |
| Vertex适配器 | `crates/router/src/adaptor/vertex.rs` | Google Vertex AI适配 |
| ZAI适配器 | `crates/router/src/adaptor/zai.rs` | ZAI协议适配 |
| 动态适配器 | `crates/router/src/adaptor/dynamic.rs` | 动态协议检测 |
| 工厂模式 | `crates/router/src/adaptor/factory.rs` | 适配器工厂 |
| E2E测试 | `crates/tests/` | 端对端测试套件 |

**验证结果:** ✅ 编译通过，协议适配完整，E2E测试基本覆盖

---

## 三、v0.4: 智能负载均衡与故障转移 — 现状分析

### 3.1 已实现功能（60%）

| 模块 | 文件路径 | 功能描述 | 完成度 |
|------|---------|---------|--------|
| 基础熔断器 | `crates/router/src/circuit_breaker.rs` | 失败计数触发熔断 | 100% |
| 智能熔断器 | `crates/router/src/smart_circuit_breaker.rs` | 错误率触发、多级熔断 | 100% |
| 健康探测 | `crates/router/src/health_probe.rs` | Half-Open主动探测 | 90% |
| 模型路由 | `crates/router/src/model_router.rs` | 模型→通道路由逻辑 | 85% |
| 亲和性缓存 | `crates/router/src/affinity.rs` | HRW哈希+双TTL缓存 | 100% |
| 通道状态追踪 | `crates/router/src/channel_state.rs` | 通道健康状态管理 | 80% |
| 响应质量检测 | `crates/router/src/response_quality.rs` | 响应质量评估 | 70% |
| 轮询负载均衡 | `crates/router/src/balancer/mod.rs` | 简单Round Robin | 100% |

### 3.2 待实现功能（40%）

| 功能 | 优先级 | 描述 |
|------|--------|------|
| 加权随机负载均衡 | 🔴 高 | 基于权重的随机选择 |
| 最少连接负载均衡 | 🔴 高 | 基于当前连接数选择 |
| 健康感知路由 | 🔴 高 | 结合健康分数的智能路由 |
| 动态权重调整 | 🟡 中 | 基于负载自动调整权重 |
| 熔断状态可视化 | 🟡 中 | 熔断状态暴露给API |
| 故障转移策略配置 | 🟡 中 | 可配置的故障转移规则 |

---

## 四、v0.4 智能负载均衡与故障转移 — 详细设计

### 4.1 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                    请求入口 (Router)                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              L1: 模型路由 (ModelRouter)                      │
│  - 模型能力匹配                                              │
│  - 通道分组过滤                                              │
│  - 优先级排序                                                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              L2: 亲和性路由 (AffinityCache)                   │
│  - HRW哈希匹配                                              │
│  - 双TTL缓存 (Sticky/Hard)                                   │
│  - 故障驱逐                                                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              L3: 智能调度器 (CombinedScheduler)               │
│  - 健康分数计算                                              │
│  - 加权评分                                                  │
│  - 熔断状态感知                                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              L4: 负载均衡器 (LoadBalancer)                    │
│  - 加权随机 (WeightedRandom)                                 │
│  - 最少连接 (LeastConnections)                               │
│  - 轮询 (RoundRobin)                                         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              L5: 故障转移 (Failover)                          │
│  - 智能熔断器检测                                            │
│  - 重试策略                                                  │
│  - 降级处理                                                  │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 核心模块设计

#### 4.2.1 负载均衡器 (LoadBalancer)

**文件路径:** `crates/router/src/balancer/mod.rs`

**新增结构体:**

```rust
/// 负载均衡策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    /// 轮询
    RoundRobin,
    /// 加权随机
    WeightedRandom,
    /// 最少连接
    LeastConnections,
}

/// 负载均衡器接口
pub trait LoadBalancer: Send + Sync {
    /// 选择下一个通道索引
    fn select(&self, candidates: &[Upstream]) -> usize;
}

/// 加权随机负载均衡器
pub struct WeightedRandomBalancer {}

impl WeightedRandomBalancer {
    pub fn new() -> Self {
        Self {}
    }
}

impl LoadBalancer for WeightedRandomBalancer {
    fn select(&self, candidates: &[Upstream]) -> usize {
        // 实现加权随机算法
        // 权重越高，被选中的概率越大
    }
}

/// 最少连接负载均衡器
pub struct LeastConnectionsBalancer {
    connections: Arc<DashMap<i32, AtomicUsize>>, // channel_id -> connection_count
}

impl LeastConnectionsBalancer {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }
    
    /// 增加连接计数
    pub fn increment(&self, channel_id: i32) {
        // ...
    }
    
    /// 减少连接计数
    pub fn decrement(&self, channel_id: i32) {
        // ...
    }
}

impl LoadBalancer for LeastConnectionsBalancer {
    fn select(&self, candidates: &[Upstream]) -> usize {
        // 选择当前连接数最少的通道
    }
}
```

**新增文件:**

| 文件 | 描述 |
|------|------|
| `crates/router/src/balancer/weighted_random.rs` | 加权随机算法实现 |
| `crates/router/src/balancer/least_connections.rs` | 最少连接算法实现 |
| `crates/router/src/balancer/trait.rs` | LoadBalancer trait定义 |

#### 4.2.2 智能调度器增强

**文件路径:** `crates/router/src/scheduler/combined.rs`

**新增字段:**

```rust
pub struct CombinedScheduler {
    // ... 现有字段 ...
    
    /// 健康分数权重
    health_weight: f64,
    /// 延迟分数权重
    latency_weight: f64,
    /// 价格分数权重
    price_weight: f64,
    /// 熔断状态权重
    circuit_weight: f64,
}
```

**新增方法:**

| 方法 | 描述 |
|------|------|
| `score_with_health()` | 结合健康状态的评分 |
| `score_with_latency()` | 结合延迟的评分 |
| `score_with_price()` | 结合价格的评分 |

#### 4.2.3 故障转移策略配置

**文件路径:** `crates/router/src/config.rs`

**新增配置:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct FailoverConfig {
    /// 是否启用故障转移
    pub enabled: bool,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔 (毫秒)
    pub retry_delay_ms: u64,
    /// 是否降级到低优先级通道
    pub allow_degrade: bool,
    /// 降级时的权重惩罚因子
    pub degrade_penalty: f64,
    /// 熔断触发后的冷却时间 (秒)
    pub cooldown_seconds: u64,
}
```

#### 4.2.4 熔断状态API

**文件路径:** `crates/server/src/api/channel.rs`

**新增端点:**

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v1/channels/{id}/circuit` | GET | 获取通道熔断状态 |
| `/api/v1/channels/{id}/circuit/reset` | POST | 手动重置熔断状态 |
| `/api/v1/channels/circuit/status` | GET | 获取所有通道熔断状态 |

**响应结构:**

```rust
#[derive(Serialize)]
pub struct CircuitStatus {
    pub channel_id: i32,
    pub channel_name: String,
    pub state: String, // "closed" | "open" | "half_open"
    pub error_rate: f64,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_failure_time: Option<String>,
    pub cooldown_remaining_seconds: u64,
}
```

### 4.3 数据库变更

**无需新增表结构** — v0.4 功能主要基于内存状态管理，现有表结构已满足需求。

### 4.4 API 变更

**新增端点:**

| 端点 | 方法 | 权限 | 描述 |
|------|------|------|------|
| `/api/v1/channels/{id}/circuit` | GET | admin | 获取通道熔断状态 |
| `/api/v1/channels/{id}/circuit/reset` | POST | admin | 手动重置熔断状态 |
| `/api/v1/channels/circuit/status` | GET | admin | 获取所有通道熔断状态 |
| `/api/v1/channels/{id}/health` | GET | admin | 获取通道健康分数 |
| `/api/v1/channels/health/status` | GET | admin | 获取所有通道健康状态 |

### 4.5 测试计划

**单元测试:**

| 测试模块 | 测试用例 |
|---------|---------|
| WeightedRandomBalancer | 权重为0不被选中、权重越高概率越大、所有权重相等等概率 |
| LeastConnectionsBalancer | 连接计数正确增减、选择连接数最少的通道、并发场景下计数正确 |
| CombinedScheduler | 健康分数影响、延迟影响、价格影响、熔断状态影响 |
| Failover | 正常请求不触发故障转移、失败时正确重试、达到最大重试次数返回错误 |

**集成测试:**

| 测试场景 | 描述 |
|---------|------|
| 熔断触发 | 模拟高错误率，验证熔断器正确打开 |
| 故障恢复 | 熔断器打开后，验证健康探测正确关闭 |
| 负载均衡 | 多通道场景下验证负载均衡策略正确性 |
| 降级处理 | 所有高优先级通道不可用时，验证降级到低优先级 |

---

## 五、实施计划

### Phase 1: 核心负载均衡算法

| 任务 | 预估工时 | 依赖 |
|------|---------|------|
| WeightedRandomBalancer 实现 | 4h | 无 |
| LeastConnectionsBalancer 实现 | 6h | 无 |
| LoadBalancer trait 重构 | 2h | 上述任务 |

### Phase 2: 智能调度器增强

| 任务 | 预估工时 | 依赖 |
|------|---------|------|
| CombinedScheduler 健康评分 | 4h | Phase 1 |
| CombinedScheduler 延迟评分 | 3h | Phase 1 |
| CombinedScheduler 价格评分 | 3h | Phase 1 |

### Phase 3: 故障转移策略

| 任务 | 预估工时 | 依赖 |
|------|---------|------|
| FailoverConfig 配置 | 2h | 无 |
| 重试逻辑实现 | 4h | Phase 2 |
| 降级处理逻辑 | 3h | Phase 2 |

### Phase 4: API 与监控

| 任务 | 预估工时 | 依赖 |
|------|---------|------|
| 熔断状态API | 4h | Phase 3 |
| 健康状态API | 3h | Phase 3 |
| 指标监控集成 | 3h | Phase 3 |

### Phase 5: 测试与验证

| 任务 | 预估工时 | 依赖 |
|------|---------|------|
| 单元测试编写 | 8h | Phase 1-4 |
| 集成测试编写 | 6h | Phase 1-4 |
| Clippy 检查 | 2h | 所有任务 |

---

## 六、风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|---------|
| 负载均衡算法性能影响 | 🟡 中 | 使用无锁数据结构，避免热点 |
| 最少连接计数不准确 | 🟡 中 | 使用原子操作，考虑网络延迟 |
| 故障转移导致请求延迟增加 | 🟡 中 | 设置合理的最大重试次数 |
| 降级策略导致成本增加 | 🟡 中 | 配置降级惩罚因子 |

---

## 七、总结

| 版本 | 状态 | 下一步 |
|------|------|--------|
| v0.1 | ✅ 完成 | 无需改动 |
| v0.2 | ✅ 完成 | 无需改动 |
| v0.3 | ✅ 完成 | E2E测试可进一步完善 |
| v0.4 | ⚠️ 进行中 | 按上述计划实现智能负载均衡 |

**v0.4 核心缺口:**
1. **加权随机负载均衡** — 当前只有简单轮询
2. **最少连接负载均衡** — 需要连接计数机制
3. **健康感知路由** — 调度器需结合健康分数
4. **故障转移策略配置** — 需要可配置的重试/降级规则
5. **熔断状态API** — 需要暴露监控端点
