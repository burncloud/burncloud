# 工业术语 ↔ 代码概念对照表

> 本文档让有网络 / 分布式背景的新成员快速定位 router 代码里的概念，反向也帮代码读者理解为什么这些设计决策不是凭空发明。
> 对应设计蓝图：[`docs/design/channel-scheduler-hqos.md`](../design/channel-scheduler-hqos.md)
> 术语来源：BGP / OSPF（路由）、IETF DiffServ（QoS）、MPLS TE（带宽预留）、SDN（控制 / 数据面分离）、TCP 拥塞控制、分布式共识。

---

## 怎么用这张表

- **看代码遇到陌生类型** → 在右列找代码概念，左列告诉你"这是工业上的哪个东西，去搜哪个 RFC / 论文"
- **设计新模块** → 在左列找最贴近的工业术语，看右列有没有已经存在的 Rust 实现可以复用，避免重复造轮子
- **PR review 命名争论** → 优先采用工业名（已完成示范：`adaptive_limit.rs → aimd_limiter.rs`，见 § 9）

---

## 1. 架构分层（SDN / 路由器三平面）

| 工业术语 | RFC / 出处 | 在 Burncloud 的代码 / 概念 | 状态 |
|---------|-----------|----------------------------|------|
| **Control Plane**（控制面） | SDN 通用 | 异步刷新策略：`exchange_rate.rs`、`price_sync.rs`，未来 `PolicyPlane` / `HealthProber` | 部分 |
| **Data Plane**（数据面） | SDN 通用 | `route_with_scheduler` 热路径、scheduler 评分 | ✅ |
| **Management Plane**（管理面） | SDN 通用 | 管理后台 / Dioxus 客户端 | ✅ |
| **Fast Path**（快路径） | 路由器通用 | 亲和命中（规划中 L3 Affinity） | 规划 |
| **Slow Path**（慢路径） | 路由器通用 | 回落到 Scorer 评分（现 `CombinedScheduler`） | ✅ |
| **RIB**（Routing Information Base） | BGP / OSPF | `channel_abilities` 全集（数据库表） | ✅ |
| **FIB**（Forwarding Information Base） | 路由器通用 | 决策快照 + 亲和缓存（规划） | 规划 |
| **Flow Cache / Session Table** | NetFlow / 防火墙 | 规划中的 `AffinityCache`（建议改名 `FlowCache`） | 规划 |
| **Adjacency Table** | OSPF / 路由器 | `ChannelStateTracker`（建议改名 `AdjacencyTable`） | ✅ |

---

## 2. QoS / 整形 / 调度（IETF DiffServ 家族）

| 工业术语 | RFC / 出处 | 在 Burncloud 的代码 / 概念 | 状态 |
|---------|-----------|----------------------------|------|
| **MF Classifier**（多字段分类器） | RFC 2475 DiffServ | 规划中 L1 染色器（落在 `service-user::resolve_traffic_class`） | 规划 |
| **DSCP / Traffic Class** | RFC 2474 | 规划中 `TrafficColor` 字段（建议改名 `Dscp`） | 规划 |
| **srTCM**（single-rate Three Color Marker） | RFC 2697 | 单速率三色整形器 | 候选实现 |
| **trTCM**（two-rate Three Color Marker） | RFC 2698 | 规划中 `ChannelBudget`（建议改名 `TrTCMShaper`），三色令牌桶 | 规划 |
| **CIR / PIR / CBS / PBS** | RFC 2697/2698 | 承诺速率 / 峰值速率 / 承诺突发 / 峰值突发——`channel_providers.rpm_cap` 与三色 reservation | 规划 |
| **Token Bucket** | 经典限流 | 规划中 `rate_budget.rs::TokenBucket` | 规划 |
| **GCRA**（Generic Cell Rate Algorithm） | ATM | Token Bucket 的等价数学表达，可作 srTCM 实现细节 | 候选 |
| **WFQ**（Weighted Fair Queuing） | QoS 经典 | `CombinedScheduler` 评分本质（建议改名 `WfqScheduler`） | ✅ |
| **PQ**（Priority Queuing） | QoS 经典 | "绿抢红"的严格优先调度策略 | 规划 |
| **HQoS**（Hierarchical QoS） | 华为 / 思科 | 整个 L1 → L4 分层栈的官方名，对应 [`docs/design/channel-scheduler-hqos.md`](../design/channel-scheduler-hqos.md) Part 2 | 规划 |
| **WRED**（Weighted Random Early Detection） | RFC 2309 | 拥塞前按颜色早丢（红用户先 503）——可选阶段 2 增强 | 候选 |

---

## 3. 拥塞适应（TCP 家族）

| 工业术语 | RFC / 出处 | 在 Burncloud 的代码 / 概念 | 状态 |
|---------|-----------|----------------------------|------|
| **AIMD**（Additive Increase / Multiplicative Decrease） | RFC 5681 | `aimd_limiter.rs::AimdController`（已完成最高 ROI 重命名） | ✅ |
| **BBR**（Bottleneck Bandwidth and RTT） | Google 2016 | 未来可选替代方案（基于带宽 × RTT 主动探测） | 候选 |
| **ECN**（Explicit Congestion Notification） | RFC 3168 | LLM 等价物：上游 429 + `x-ratelimit-remaining` header | ✅ |
| **Exponential Backoff + Jitter** | AWS Architecture Blog | `circuit_breaker.rs` 冷却 + 随机化 | ✅ |

---

## 4. 健康 / 故障检测

| 工业术语 | RFC / 出处 | 在 Burncloud 的代码 / 概念 | 状态 |
|---------|-----------|----------------------------|------|
| **BFD**（Bidirectional Forwarding Detection） | RFC 5880 | 规划中 `health_prober.rs`（建议改名 `bfd.rs`），主动探测 | 规划 / DEFER |
| **ECMP**（Equal Cost Multi-Path） | 路由器通用 | Top-5 候选列表（建议改名 `FibBackupPaths` / `LfaList`） | ✅ |
| **LFA**（Loop-Free Alternates） | RFC 5286 | failover 备份候选 | ✅ |
| **Circuit Breaker** | Netflix Hystrix | `circuit_breaker.rs`（5 次失败 / 30s 冷却） | ✅ |
| **Hedged Requests** | The Tail at Scale (Dean & Barroso) | 同请求打两后端取先到（Enterprise OrderType 候选实现） | 候选 |

---

## 5. 调度 / 选择算法

| 工业术语 | 出处 | 在 Burncloud 的代码 / 概念 | 状态 |
|---------|------|----------------------------|------|
| **Rendezvous Hashing (HRW)**（Highest Random Weight） | Thaler & Ravishankar 1998 | 规划中 `affinity.rs` 的 user → channel 映射核心算法 | 规划 |
| **Consistent Hashing** | Karger et al. 1997 | HRW 的替代方案（候选集大且稳定时更适合） | 备选 |
| **Maglev Hashing** | Google 2016 | 候选集大、需要均匀分布时的方案 | 备选 |
| **Power of Two Choices (P2C)** | Mitzenmacher 2001 | 简单负载均衡的极小开销方案 | 参考 |

---

## 6. 联邦 / Peering（未来 BPP / 阶段 5 — DEFER）

| 工业术语 | RFC / 出处 | 在 Burncloud 的代码 / 概念 | 状态 |
|---------|-----------|----------------------------|------|
| **AS / Peer / Neighbor** | BGP | Burncloud 节点 / 对等方 | DEFER |
| **Route Announcement / Withdrawal** | RFC 4271 BGP | slot 上线 / 下线 | DEFER |
| **NEXT_HOP / AS_PATH / LOCAL_PREF / MED / BGP Community** | RFC 4271 | slot 路由属性 | DEFER |
| **Route Reflector** | RFC 4456 | 中心化 slot 目录 | DEFER |
| **RPKI**（Resource Public Key Infrastructure） | RFC 6480 | slot 通告签名验证 | DEFER |

---

## 7. 信任 / 一致性

| 工业术语 | 出处 | 在 Burncloud 的代码 / 概念 | 状态 |
|---------|------|----------------------------|------|
| **mTLS / PKI** | TLS 1.3 (RFC 8446) | 阶段 5 peer 鉴权 | DEFER |
| **Raft** | Ongaro & Ousterhout 2014 | 强一致配额（防超卖）—— 阶段 4 候选 | DEFER |
| **CRDT**（Conflict-free Replicated Data Type） | Shapiro et al. 2011 | 最终一致用量统计 —— 阶段 4 候选 | DEFER |
| **SWIM** | Das et al. 2002 | 去中心化成员检测 —— 阶段 4 候选 | DEFER |

---

## 8. 客户诉求建模（市场微观结构借词）

> **声明**：Burncloud 不是交易所。这些术语**仅借用名字**，不引入撮合 / 清算 / 手续费等交易所机制。

| 工业术语 | 出处 | 在 Burncloud 的代码 / 概念 | 状态 |
|---------|------|----------------------------|------|
| **Order Type — Limit** | 证券交易所 | 规划中 `OrderType::Budget { max_price }`，省钱客户硬上限 | 规划 |
| **Order Type — Stop-Limit** | 证券交易所 | 规划中 `OrderType::Value { target, ceiling }`，性价比客户两层 tier | 规划 |
| **Order Type — Market** | 证券交易所 | 规划中 `OrderType::Enterprise { redundancy }`，企业客户不管价格 | 规划 |
| **Order Book**（透明度思想） | 证券交易所 | 阶段 5 BPP slot 全透明的灵感来源 | DEFER |
| **RFQ**（Request for Quote） | 场外市场 / Hashflow | "客户问价 → 供应商响应"模式，未来可作 BPP 的协议变体 | 参考 |

---

## 9. 优先落地的 10 个重命名（路线图锚点）

> 来自 [设计蓝图附录 A](../design/channel-scheduler-hqos.md#优先落地的-10-个重命名按-roi)。审查决策 E-D8：先做 1 个最高 ROI（`adaptive_limit → aimd_limiter`），视 PR review 体验决定是否继续其他。第 1–4 项已落地，其余按团队接受度推进。

| # | 现名 | 建议新名 | ROI | 状态 |
|---|------|---------|-----|------|
| 1 | ~~`adaptive_limit.rs`~~ | `aimd_limiter.rs` | ⭐⭐⭐ 最高 | ✅ 已落地 |
| 2 | ~~`AdaptiveRateLimit`~~ | `AimdController` | ⭐⭐⭐（与 1 同 PR） | ✅ 已落地 |
| 3 | ~~`AdaptiveSnapshot`~~ | `AimdSnapshot` | ⭐⭐⭐（与 1 同 PR） | ✅ 已落地 |
| 4 | ~~`AdaptiveLimitConfig`~~ | `AimdConfig` | ⭐⭐⭐（与 1 同 PR） | ✅ 已落地 |
| 5 | `ChannelStateTracker` | `AdjacencyTable` | ⭐⭐ | 待评估 |
| 6 | `AffinityCache`（规划） | `FlowCache` | ⭐⭐（MVP 直接用新名） | 待 MVP |
| 7 | `ChannelBudget`（规划） | `TrTCMShaper` | ⭐⭐（阶段 2 直接用新名） | 待阶段 2 |
| 8 | `TrafficColor` | `Dscp` | ⭐ | 待评估 |
| 9 | `CombinedScheduler` | `WfqScheduler` | ⭐ | 待评估 |
| 10 | `health_prober.rs`（规划） | `bfd.rs` | ⭐（阶段 3 解 BLOCK 后直接用新名） | 待阶段 3 |

---

## 维护说明

- **新增术语规则**：只收录已经在代码或路线图中**实际出现**的概念。纯学术术语不收。
- **改名规则**：每次实际改名落地一个，回来把"建议新名"列移到"现名"，并标 ✅。
- **本表与 [`docs/design/channel-scheduler-hqos.md`](../design/channel-scheduler-hqos.md) 协同**：那篇讲"为什么这样设计 / 哪个阶段实施"，本表讲"代码里叫什么 / 工业上的官方术语是什么"。
- **PR review 引用**：评 PR 时若涉及命名争论，可直接 `link to docs/code/GLOSSARY.md#3-拥塞适应tcp-家族` 提供权威依据。
