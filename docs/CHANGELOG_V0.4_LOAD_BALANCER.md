# v0.4 智能负载均衡与故障转移 - 修改说明

> **版本**: v0.4.0
> **日期**: 2026-07-24
> **分支**: `feature/load-balancer-v0.4`
> **状态**: 待合并

---

## 一、修改概述

本次修改实现了 v0.4 版本的核心功能：**智能负载均衡与故障转移**。具体包括：

| 功能模块 | 修改类型 | 状态 |
|---------|---------|------|
| 负载均衡器接口 | 新增 | ✅ 完成 |
| RoundRobin 算法 | 重构 | ✅ 完成 |
| WeightedRandom 算法 | 新增 | ✅ 完成 |
| LeastConnections 算法 | 新增 | ✅ 完成 |
| 故障转移配置 | 新增 | ✅ 完成 |

---

## 二、详细修改内容

### 2.1 文件变更清单

| 文件路径 | 修改类型 | 说明 |
|---------|---------|------|
| `crates/router/Cargo.toml` | 修改 | 添加 rand 依赖 |
| `crates/router/src/balancer/mod.rs` | 新增 | 完整实现三种负载均衡算法 |
| `crates/router/src/config.rs` | 修改 | 新增 FailoverConfig 配置结构 |
| `docs/VERSION_STATUS.md` | 新增 | 版本状态分析文档 |

### 2.2 新增模块与功能

#### 2.2.1 负载均衡器接口

**文件**: `crates/router/src/balancer/mod.rs`

**新增枚举**: `LoadBalanceStrategy`
```rust
pub enum LoadBalanceStrategy {
    RoundRobin,      // 轮询策略
    WeightedRandom,  // 加权随机策略
    LeastConnections, // 最少连接策略
}
```

**新增 Trait**: `LoadBalancer`
```rust
pub trait LoadBalancer: Send + Sync {
    fn select(&self, candidates: &[Upstream]) -> usize;
    fn strategy_name(&self) -> &str;
}
```

**新增结构体**: `Upstream`（内部使用）
```rust
pub struct Upstream {
    pub id: String,
    pub name: String,
    pub weight: usize,
}
```

#### 2.2.2 三种负载均衡算法

| 算法 | 结构体 | 时间复杂度 | 描述 |
|------|--------|-----------|------|
| **轮询** | `RoundRobinBalancer` | O(1) | 基于原子计数器，线程安全 |
| **加权随机** | `WeightedRandomBalancer` | O(n) | 线性扫描加权随机选择，支持零权重处理 |
| **最少连接** | `LeastConnectionsBalancer` | O(n) | 基于 DashMap 的连接计数，支持并发递增/递减 |

**RoundRobinBalancer**:
- 使用 `AtomicUsize` 实现全局原子计数器
- 原子操作保证线程安全
- 简单高效，O(1) 时间复杂度

**WeightedRandomBalancer**:
- 线性扫描加权随机算法
- 权重为0的通道不被选中
- 所有权重为0时均匀随机选择

**LeastConnectionsBalancer**:
- 使用 `DashMap<String, AtomicUsize>` 存储通道连接数
- 提供 `increment()`/`decrement()`/`get_connections()` 方法
- 连接数相同时随机选择

#### 2.2.3 故障转移配置

**文件**: `crates/router/src/config.rs`

**新增结构体**: `FailoverConfig`
```rust
pub struct FailoverConfig {
    enabled: bool,           // 是否启用故障转移（默认true）
    max_retries: u32,        // 最大重试次数（默认2）
    retry_delay_ms: u64,     // 重试间隔（默认100ms）
    allow_degrade: bool,     // 是否允许降级到低优先级通道（默认true）
    degrade_penalty: f64,    // 降级惩罚因子（默认0.5）
    cooldown_seconds: u64,   // 熔断冷却时间（默认30s）
}
```

**特性**:
- 支持 JSON 序列化/反序列化
- 所有字段有合理的默认值
- 使用 `#[serde(default = "...")]` 支持部分配置

---

## 三、依赖变更

### 3.1 新增依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `rand` | 0.8 | 加权随机和最少连接算法中的随机选择 |

### 3.2 修改文件

**文件**: `crates/router/Cargo.toml`
```toml
[dependencies]
# ... 现有依赖 ...
rand.workspace = true
```

---

## 四、技术特点

### 4.1 线程安全

所有负载均衡器实现 `Send + Sync` trait：
- 使用 `DashMap` 替代 `HashMap` 实现并发安全
- 使用 `AtomicUsize` 替代普通 `usize` 实现原子操作
- 使用 `Arc` 实现共享所有权

### 4.2 边界处理

| 边界场景 | 处理方式 |
|---------|---------|
| 空候选集 | 返回索引0 |
| 单候选集 | 始终返回索引0 |
| 零权重 | 跳过该通道或均匀随机 |
| 连接数下溢 | 防止计数器变为负数 |
| 并发竞争 | 原子操作保证正确性 |

### 4.3 可扩展性

- `LoadBalancer` trait 定义统一接口，便于扩展新算法
- `FailoverConfig` 支持 JSON 配置，便于动态调整
- 所有字段有默认值，支持渐进式配置

---

## 五、编译验证

### 5.1 编译结果

- 运行命令: `cargo build --package burncloud-router`
- 结果: ✅ 通过（无错误）

### 5.2 Clippy 检查

- 运行命令: `cargo clippy --package burncloud-router -- -D warnings`
- 结果: ✅ 通过（无错误）
- 注意: 新增类型添加了 `#[allow(dead_code)]` 标注，因为尚未被其他模块引用

---

## 六、后续工作

### 6.1 待实现功能

| 功能 | 优先级 | 描述 |
|------|--------|------|
| 负载均衡器集成到路由核心 | 🔴 高 | 将新算法集成到 ProxyLogic |
| 熔断状态API | 🟡 中 | 暴露熔断状态监控端点 |
| 健康状态API | 🟡 中 | 暴露通道健康分数端点 |
| 动态权重调整 | 🟡 中 | 基于负载自动调整权重 |

### 6.2 集成计划

1. **Phase 1**: 将 `LoadBalancer` 集成到 `ModelRouter`
2. **Phase 2**: 根据策略配置选择对应的负载均衡器
3. **Phase 3**: 将 `FailoverConfig` 集成到重试逻辑
4. **Phase 4**: 实现熔断状态和健康状态API

---

## 七、代码评审要点

### 7.1 重点关注

1. **并发安全性**: 所有共享状态是否使用正确的并发原语
2. **边界条件**: 空候选集、零权重、连接数下溢等场景
3. **性能**: 负载均衡算法的时间复杂度和内存占用

### 7.2 风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|---------|
| 并发竞态条件 | 🟡 中 | 使用原子操作和 DashMap |
| 内存泄漏 | 🟡 中 | 使用 Arc 自动释放 |
| 权重计算溢出 | 🟢 低 | 使用 usize 和合理边界检查 |

---

## 八、提交信息

```
feat: 实现v0.4智能负载均衡与故障转移

实现三种负载均衡算法：
- RoundRobin（轮询）：基于原子计数器的线程安全实现
- WeightedRandom（加权随机）：线性扫描加权选择，支持零权重处理
- LeastConnections（最少连接）：基于DashMap的连接计数，支持并发操作

新增故障转移配置：
- FailoverConfig：支持重试次数、间隔、降级策略、冷却时间配置
- 支持JSON序列化/反序列化，所有字段有默认值

技术特点：
- 所有组件实现Send+Sync，支持并发环境
- 使用DashMap和AtomicUsize保证线程安全
- 线性扫描算法适用于小规模候选集（路由场景典型）
```
