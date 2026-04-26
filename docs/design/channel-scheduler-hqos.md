# 多渠道调度器：华为路由器视角 × 多供应商直连路由

> **文档定位**：设计参考文档（design blueprint），**非立即实施计划**。
> **状态**：蓝图（Living Reference）。MVP / 阶段 2 已经过审查并条件性 APPROVE；阶段 3 / 3+ / 4 / 5 已 DEFER 至 backlog，**不在 12 月路线图承诺内**。
> **来源**：Issue [#143](https://github.com/burncloud/burncloud/issues/143)
> **关联**：审查决策见末尾「Ready 审查衔接」一节；术语对照见 `docs/code/GLOSSARY.md`。

---

## Context

**问题起源**：Burncloud 是一个多渠道 LLM 网关代理平台。当同一模型（如 glm-5.1）有多个上游渠道（阿里云渠道1 + z.ai 渠道2）、多个用户并发访问时，当前调度器出现"亲和性问题"——同一用户的多轮对话被分散到不同渠道，导致 KV cache 失效、上下文不连续、计费不连贯，并且难以在"价格最优 / RPM 不超限 / 会话亲和"三目标间取得平衡。

**用户提的真正问题**："如果让华为路由器工程师来设计，他们会如何思考？"——这是一个以设计哲学驱动架构的问题，答案不是堆砌组件，而是用一套成熟的分层思想重新审视调度。

**部署形态假设**：当前单实例部署，未来可能水平扩容。因此阶段 2 建议**一开始就抽 `BudgetBackend` trait**（内存实现即可），但不落地 Redis 实现；等真正多实例再补 `rate_budget_redis.rs`。这样未来切换零重写。

**预期产出**：（1）一套可追溯到华为路由器设计哲学的调度器分层架构；（2）与现有代码衔接的改造蓝图；（3）分阶段落地路线（未来可选执行）。

---

## Strategic Vision：多供应商直连路由，面向"便宜 + 稳定"

**终极定位**：**不**是 LLM 网关、**不**是交易所，而是**多供应商直连路由平台**——
- 供应商透明上架账号 → 客户直通一手资源 → 消灭转售层
- 核心目标：让用户用到**便宜 + 稳定**的 LLM 访问
- **不做**撮合引擎、清算、手续费体系、市场监察等交易所重设施——那些是商业目标的配套，不是当前技术目标

"交易所"语汇只作为**设计灵感**，不作为产品边界。仅借用两个概念：
1. **透明度（Order Book 思想）**：供应商的 slot 全部透明，对应阶段 5 BPP
2. **Order Type（订单类型）**：让每个客户按自己的诉求调度

### 三种客户诉求 = 三种 Order Type

直接对应"便宜 + 稳定"的不同组合：

| 客户类型 | 诉求 | Order Type | 调度行为 |
|---|---|---|---|
| 省钱客户 | 只要便宜，贵的宁可失败 | **Limit**（价格硬上限） | 过滤贵渠道；全爆时返回 503，不 fallback 到贵的 |
| 性价比客户 | 先便宜，不行 fallback 到上限内 | **Stop-Limit**（目标价 + 保护价） | 两层 tier，tier1 爆满降级到 tier2 |
| 企业客户 | 只要稳定，价格次要 | **Market**（不管价格） | 预留容量 + hedged 冗余请求 |

### 三个正交维度（未来逐步叠加）

- **Order Type** = 执行逻辑（上述三种）
- **Trader Class** = 客户门类（VIP/付费/免费 → 手续费、优先级加成，**非 MVP**）
- **QoS Color** = 内部调度（整形/排队/抢占）

**MVP 边界**：Order Type + 亲和 + 染色。Trader Class 推迟；撮合/清算/监察一类交易所支柱不做。

---

## 现状快照（已调研）

**已有能力**（保留）：

- `crates/router/src/model_router.rs` `ModelRouter::route_with_scheduler` — 调度入口
- `crates/router/src/scheduler/{mod.rs, combined.rs, passthrough.rs}` — Combined（health × cost × rpm 三因子评分）与 Passthrough（加权轮询）
- `crates/router/src/adaptive_limit.rs` — 从 429 被动学习 RPM/TPM
- `crates/router/src/channel_state.rs` — `ChannelStateTracker` 健康分数
- `crates/router/src/circuit_breaker.rs` — 熔断（5 次失败 / 30s 冷却）
- Top-5 候选 failover 循环（`lib.rs` 内）
- `crates/database/crates/router-log/` `router_logs` 表（upstream_id、user_id、cost、9 种 token 字段）
- `crates/database/crates/channel/` `channel_providers` + `channel_abilities` 多对多能力表
- `crates/database/crates/billing/` `billing_prices` 多模态定价表

**核心缺失**：

- 无会话亲和性（user → channel 绑定缓存）
- 无 TPM 预算，RPM 仅靠 429 被动学习
- channel 表无 RPM 上限硬配置字段
- 无用户分级（VIP / 付费 / 免费）差异化调度
- 无渠道主动健康探测
- 无多实例共享的全局 RPM 账本

---

## Part 1 — 华为工程师的六条灵魂哲学（映射到 LLM 调度）

### 哲学一：控制面 / 数据面严格分离（SDN）

路由器不会在每个报文到达时才去计算路由表。**当前 `CombinedScheduler::score` 在每次请求都做价格查询、汇率换算、健康查询、adaptive 查询——这是把控制面计算塞进了数据面热路径**。应改为：策略推演（价格、健康、容量）在控制面异步刷新，数据面只查一次预烘焙的决策快照。

### 哲学二：HQoS 分层调度

华为框式路由器出接口调度是 4 层：**流分类 → 流量整形（令牌桶）→ 队列调度（PQ + WFQ）→ 端口**。LLM 等价结构：**用户分类（VIP / 付费 / 免费）→ RPM/TPM 令牌桶整形 → 渠道队列调度（亲和优先 + 评分）→ upstream**。现在的 Combined 只做到了第 3 层的评分部分，第 1、2 层完全缺失。

### 哲学三：DiffServ 三色标记（Guaranteed / Assured / Best-Effort）

入口把请求染色，下游所有策略只看颜色。**企业 = 绿（保证容量、预留渠道、永不 429）、付费 = 黄（软保证，可借用绿的空闲）、免费 = 红（尽力而为，拥塞时先丢）**。染色一次，后续调度器 / 限流器 / 熔断器 / failover 只消费颜色字段——用染色替代"到处透传用户等级"的复杂耦合。

### 哲学四：快慢路径分离 + 会话亲和"粘而不僵"

路由器 flow cache：同一 5 元组走 fast path，未命中才重查 FIB；但缓存带老化 + 负载感知，不会僵死。**LLM 等价：user_id → channel_id 用一致性哈希 / Rendezvous Hash 建稳定映射，但当目标 channel 进入 Cooldown 或 RPM 用量 > 80% 时，亲和层主动让位 slow path**。亲和不是硬绑定，是一个带退让条件的提示。

### 哲学五：BFD 式主动探测

BFD 以 50ms 级打探测包，不等数据流失败才收敛。**当前 adaptive_limit 完全被动**——必须有真实流量撞到 429 才能学习，流量低谷时冷门渠道会变成定时炸弹。应增加轻量主动探测（每 30s 对每 channel 打 max_tokens=1 的小 prompt），预先感知退化。

### 哲学六：预留 + 抢占 + 借用（MPLS TE 带宽语义）

MPLS TE 的带宽不是"先到先得"：各等级预留一部分，允许借用空闲额度，拥塞时按优先级抢占。**映射到 RPM**：例如 glm-5.1 渠道 1 RPM=100 → Guaranteed 预留 40 / Assured 40 / Best-Effort 20。低优先级可借用高优先级余量，高优先级一来立即抢回。这是"5 个用户抢 2 个渠道"问题的系统答案。

---

## Part 2 — 分层架构（控制面 / 数据面）

```
                ┌───────────── 控制面（慢，异步刷新）─────────────┐
                │ PolicyPlane   策略配置 + 三色预留比例             │
                │ HealthProber  主动 BFD 式探测（tokio interval）   │
                │ PriceCache    价格表后台刷新                       │
                │ BudgetSync    多实例 Redis 账本协调（阶段 4）      │
                └──────────┬────────────────────────────────────────┘
                           ▼ ArcSwap 发布决策快照
┌──────────────────────── 数据面（快，sub-ms）────────────────────────┐
│ L1 Classifier    请求染色 GREEN/YELLOW/RED（来自 user_tier）          │
│ L2 Shaper        per-(channel, color) 令牌桶；预留 / 借用 / 抢占判定   │
│ L3 Affinity (Fast) Rendezvous Hash + AffinityCache；命中且健康直接返回 │
│ L4 Scorer  (Slow) 保留现 CombinedScheduler；亲和未命中 / 降级时触发    │
│ L5 Failover      保留现 Top-5 循环 + CircuitBreaker；失败驱逐亲和缓存   │
│ L6 Observability 每层 counter 写入 router_logs（layer_decision 字段）  │
└────────────────────────────────────────────────────────────────────┘
```

**关键设计要点**：

- **不替换 Combined 调度器**：它正是 L4 的正确实现。改造方式是"上叠 L3 亲和层 + 下垫 L2 整形层 + 旁挂 L6 探测 + 入口染色"。
- **亲和选 Rendezvous Hash（HRW）**：候选集小（≤ 20）、频繁变更（健康波动），HRW 比一致性哈希简洁、候选增删影响范围小。`score_i(u) = hash(user_id, channel_id_i) × weight_i × health_i`，取 argmax。
- **双 TTL 亲和**：粘性 TTL 5 min（优先命中）+ 硬过期 30 min（强制重选）。失败即驱逐。
- **TPM 预估扣 + 事后补差**：请求发起时按历史均值扣，响应回来后读 router_logs 做差值补偿。
- **请求维度与候选维度分离**（审查决策 D6 / E-D2）：原 issue 提议向 `SchedulingContext` 加 `color / user_id / order_type`；审查后改为**新建 `SchedulingRequest`** 容纳请求元数据，与 `SchedulingContext`（候选因子快照）解耦，保持 `ChannelScheduler` trait 签名不变。
- **调度器本身保持颜色无关**：颜色只被 L2 Shaper 消费——这是 DiffServ 的精髓。
- **Classifier 归属**（审查决策 E-D3）：原 issue 把 `classifier.rs` 放在 router crate；审查后改为**在 `burncloud-service-user` 增加 `resolve_traffic_class(user_id) → TrafficColor`**，Server 层调用后通过 `SchedulingRequest` 注入 router，避免 router 反向依赖 user 数据。

---

## Part 3 — 分阶段实施

### 阶段 MVP（1–2 周）：亲和层 + 三色染色 + Order Type

**目标**：解决"5 用户 2 渠道摇摆"痛点 + 把"便宜 / 性价比 / 稳定"三种客户诉求真正翻译成调度行为；单实例部署即可。

**关键文件**：

- 新增 `crates/router/src/order_type.rs`（`enum OrderType { Budget { max_price }, Value { target, ceiling }, Enterprise { redundancy } }` + 每种类型的候选过滤 / 排序函数）
- 新增 `crates/router/src/affinity.rs`（HRW + DashMap AffinityCache，双 TTL）
- 新增 `crates/router/src/rate_budget.rs`（`BudgetBackend` trait + `InMemoryBudget`，**只做借用，抢占延后**——审查决策 E-D6）
- 在 `burncloud-service-user` 添加 `resolve_traffic_class(user_id) → TrafficColor`（**不**新增 `crates/router/src/classifier.rs`，按审查决策 E-D3）
- 修改 `crates/router/src/model_router.rs`（`route_with_scheduler` 内插入顺序：**OrderType 过滤 → Affinity → Scorer**——按审查决策 D7 顺序，OrderType 优先于 Affinity；failover 时驱逐失败条目）
- 修改 `crates/router/src/scheduler/mod.rs`（**拆分**为 `SchedulingRequest`（请求维度：`user_id / color / order_type`）+ `SchedulingContext`（候选维度：`factors`）；`ChannelScheduler` trait 不动——审查决策 D6 / E-D1 / E-D2）

**DB 变更**（合并为单批次 migration——审查决策 E-D9）：

- `router_tokens` 或用户表增加 `order_type VARCHAR(16)`（默认 `"value"`）+ `price_cap_nanodollars BIGINT NULL`
- `channel_providers` 同时新增 `rpm_cap / tpm_cap / reservation_{green,yellow,red}` 5 字段（原属阶段 2，提前合并）
- `router_logs` 同时新增 `layer_decision / traffic_color` 字段（原属阶段 3，提前合并）
- 全部 SQL 必须使用 `ph(db.is_postgres(), N) / phs(...)` 占位符，禁止硬编码 `$1 / ?`

**HTTP 契约**（审查决策 D12）：

- Shaper 主动拒绝时返回 **503**（不是 429），并带 `X-Rejected-By: shaper` header + `Retry-After: <seconds>`，便于客户端区分"上游 429"与"本地 Shaper 拒绝"

**前置条件**（审查决策 D3）：

1. **router_logs ad-hoc 数据分析**——证明"5 用户 2 渠道摇摆"痛点真实存在并可量化（同 user_id 在最近 N 天被路由到不同 channel 的方差分布）
2. 现有代码 SQL 占位符 migrate 到 `ph/phs`（独立清理 PR）
3. `NoAvailableChannelsError` 改 `thiserror` derive（独立清理 PR）

**验证**：

- 固定 user_id 在 10 次调用中命中同 channel ≥ 9 次；熔断后自动切换
- Budget 客户：贵渠道从候选池消失；便宜渠道全爆时返回 503 而非 fallback
- Enterprise 客户：便宜渠道可用时仍优先高健康分渠道
- 染色 / OrderType 路径在 router_logs 可追溯

---

### 阶段 2（2–3 周）：三色令牌桶 Shaper + RPM 硬配置

**目标**：主动整形替代被动学习 RPM，支持预留 / 借用 / 抢占。

**部署假设**：单实例为主。但**必须一开始就抽 `BudgetBackend` trait**（参见 Part 4 取舍 2），内存后端为默认实现；未来多实例部署时直接补 Redis 后端，无需重写。

**关键文件**：

- `crates/router/src/rate_budget.rs`：MVP 已抽 trait `BudgetBackend { try_consume, refund, snapshot }`；本阶段实现 `InMemoryBudget` 完整 `ChannelBudget { rpm_buckets: [TokenBucket; 3], tpm_buckets: [TokenBucket; 3] }`，`try_consume(color)`、`borrow_from(higher)`，**抢占在本阶段后半段或独立 PR 落地**
- 修改 `crates/router/src/model_router.rs`（调度选中后、发请求前 `consume` RPM；响应后 `consume` TPM 差值）
- 修改 `crates/router/src/aimd_limiter.rs`（学到的 limit 反哺 `ChannelBudget::total_rpm_cap`，未硬配置时作为缺省值）

**DB 变更**：已在 MVP 合并落地，无新增。

**验证**：压测下 RPM 超限前 Shaper 先拒（router_logs 新增 `rejected_by_shaper` 原因）；真实 429 命中率 < 0.1%；抢占演示（绿用户来时红被优先放弃）。

---

### 阶段 3（1–2 周）：主动探测 + 观测增强 — **DEFER**

> **审查决策 D8**：阶段 3 主动探测 BLOCK 直到 ToS 合规澄清完成。理由：作者在阶段 3+ 强调"不主动试探、不违反上游 ToS"，但阶段 3 的 `health_prober` 本身就是主动探测，逻辑内部存在张力，需法务 / ToS 评估。

**目标**：消除冷启动盲区；全链路可观测。

**关键文件**：

- 新增 `crates/router/src/health_prober.rs`（tokio interval 30s，对每 channel 的 weight 最大的模型打 max_tokens=1 探针，写回 `ChannelStateTracker`）
- 修改 `crates/database/crates/router-log/` 给 router_logs 加上游指纹字段

**前置工程契约**（审查决策 E-D7）：

1. 探针流量 HTTP header 带 `X-Burncloud-Probe: true` 自我披露
2. 探针频率配置化（默认 30s，运维可调到 1h 或关闭）
3. 每 channel 可独立禁用探测
4. 探针失败不进入 `adaptive_limit`（防止探测策略污染真实限流学习）

**DB 变更**（已与 MVP 合并的 `layer_decision / traffic_color` 之外，本阶段补充）：router_logs 加上游指纹字段——

- `system_fingerprint VARCHAR(64)` — OpenAI 返回的上游配置指纹
- `upstream_request_id VARCHAR(128)` — 上游 x-request-id，后续聚类识别底层账号
- `ttft_ms INT` — time to first token，延迟分布聚类用
- `upstream_ratelimit_remaining INT` — 429 前兆的 ECN 信号

**验证**：Grafana 看板分层命中率；冷 channel 离线 2 min 内被探测发现。

---

### 阶段 3+（并行可做，零对抗）：上游指纹识别与号池推断 — **DEFER**

**目标**：利用规模带来的信息不对称，识别上游渠道是单账号还是号池、号池中有几个底层账号。**纯观察 + 离线分析**，不主动试探，不违反上游 ToS。

**三步演化**：

1. **被动采集期（零成本）**：阶段 3 已加的指纹字段持续写入 router_logs，跑 2–4 周
2. **离线聚类期**：每 channel 对 (`ttft_ms`, `system_fingerprint`, `upstream_request_id` 前缀) 跑 DBSCAN，打标签 `single_account / pool_N / unknown`
3. **轻量主动探测期**（可选）：对 `pool_*` 渠道每天几十次轻探针刻画池内账号数

**价值**：

- 为阶段 5 BPP 谈判提供数据筹码（"我知道你这条线后面是 12 个小号池"）
- 为调度器提供更精准的亲和 key（对号池 channel 可基于请求指纹做 sub-channel 亲和）
- 早期识别超卖 / 降级 / 切换账号的供应商

**关键文件（新增）**：

- `crates/router/src/fingerprint/collector.rs`（响应头指纹提取）
- `crates/router/src/fingerprint/cluster.rs`（离线 DBSCAN，读 router_logs，写 channel 标签）

**验证**：离线跑 DBSCAN，对已知号池 channel 能稳定聚出 ≥ 2 簇。

---

### 阶段 4（按需触发）：多实例分布式账本 — **DEFER**

> **审查决策 D4**：YAGNI，等真实水平扩容需求。MVP 阶段已抽好 `BudgetBackend` trait，未来切换零重写。

**目标**：K8s 水平扩容下 RPM 账本一致。

**关键文件**：

- 抽 trait `BudgetBackend`（已在 MVP 完成），新增 `crates/router/src/rate_budget_redis.rs`（Lua `DECR_IF_POSITIVE` 脚本）
- 新增 `crates/router/src/prober_lease.rs`（Redis SETNX leader 选举，避免 N 实例重复探测）
- AffinityCache 走 Redis（保证跨实例粘性）

**DB 变更**：无。引入 Redis 依赖。

**验证**：2 实例并发压满，Redis 聚合 RPM 与上游 429 率吻合。

---

### 阶段 5（长期愿景，3–6 个月工程）：BPP 联邦化 / Burncloud Peering Protocol — **DEFER**

> **审查决策 D4 / D9 / E-D10**：等供应商 buy-in 商业验证（不是技术问题）。若未来落地：crate 必须命名 `burncloud-bpp`（符合 workspace `burncloud-*` 约定）；必须 `feature = "bpp"` gate，默认不编译，避免拖慢主 binary 的 `cargo check`。

**目标**：让供应商的 Burncloud 节点把 slot 资源暴露给客户的 Burncloud 节点，实现端到端 slot 级亲和。**等价于 LLM 容量网络的 BGP**。

**场景**：供应商 A 电脑跑 Burncloud + 注册 10 个 z.ai 账号；客户 1 / 客户 2 的 Burncloud 分别连接到 A 的 Burncloud。A 把 10 个 slot 作为独立子渠道暴露给客户，客户可对每个 slot 做独立亲和。

**核心架构**：

```
[客户 Burncloud]  ──pull/订阅──▶  [供应商 Burncloud]  ──▶ 10 个 z.ai 账号
       │                                    │
       ├─ BPP Importer                      ├─ BPP Exporter
       ├─ slot 物化到本地 RIB                 ├─ Slot ID 混淆器（per-peer 轮换）
       ├─ FIB 直连 slot 级亲和                ├─ Slot 健康推送（WebSocket）
       └─ mTLS + RPKI 式签名验证              └─ 路由聚合 / 明细模式切换
```

**关键设计点**：

- **Slot 不透明 ID**：per-peer、per-hour 重新 hash，防止客户反向识别底层账号
- **打包定价 + 分散计费**：供应商按池报价（防客户挑 slot），计费按 slot 级（便于审计）
- **强制广播全集**：客户接入必须接受全部 slot，防止 adverse selection
- **路由撤销实时**：slot 故障通过 WebSocket 毫秒级推送
- **RPKI 式签名**：防伪造 slot 通告

**关键文件（新增 crate `burncloud-bpp`）**：

- `crates/bpp/src/exporter.rs`（供应商侧：本地 channel → 对外 slot 清单）
- `crates/bpp/src/importer.rs`（客户侧：拉取 peer slot → 物化到本地 RIB）
- `crates/bpp/src/slot_obfuscator.rs`（opaque ID 轮换）
- `crates/bpp/src/peer_registry.rs`（peer 认证 + 公钥管理）
- `crates/bpp/src/route_announce.rs`（BGP 式通告 / 撤销协议）

**DB 变更**：

- 新增 `bpp_peers` 表（peer_id, endpoint, pubkey, status, since）
- 新增 `bpp_imported_slots` 表（slot_opaque_id, peer_id, model, rpm_cap, expires_at）
- `channel_providers` 增加 `origin` 字段（`local / federated_via_{peer_id}`）

**渐进式落地路径（避免一步到位）**：

1. **静态路由阶段**：手动在配置文件把 peer 的账号列为 channel（不动协议，先跑通心智模型）
2. **RIP 式原型**：两节点定时全量同步 slot 清单（最朴素，先验证正确性）
3. **BGP 式增量通告**：仅变更、带版本号、撤销、签名（生产级）

**商业权衡**：

- 供应商愿意暴露的激励：透明换议价 / 高利用率 / 故障隔离精准
- 对抗客户反向识别的护栏：混淆 slot ID + 定期轮换 + 打包定价
- **不建议走对抗路线**（破解黑盒号池）——违反 ToS、陷入军备竞赛

**验证**：两台机器互联，A 暴露 3 个 slot，B 验证亲和命中率 ≥ 95%；A 主动撤销 slot_2，B 在 500 ms 内停止路由。

---

## Part 4 — 关键取舍与风险

1. **亲和 vs 成本最优的冲突**。HRW 选中的 channel 若 cost 因子比当前最优差 > 30%，主动触发 slow path 重选；否则保持亲和（阈值可调）。对应哲学四"粘而不僵"。
2. **单实例 DashMap vs 多实例 Redis**。阶段 1–2 坚持单实例账本；多实例部署时先用会话保持约束"单 writer"；阶段 4 才上 Redis，避免过早分布式复杂度。
3. **三色预留比例静态 vs 动态**。采用"静态预留 + 动态借用 + 软抢占"——低优先级可借用高优先级余量，高优先级需要时立即抢回（不中断已发出的请求）。对应哲学六。
4. **主动探测的成本**。只探测每 channel 的主力模型、max_tokens=1，且探测流量计入 Best-Effort 桶，防止挤占用户流量。
5. **亲和 TTL 设置**。双 TTL（5 min 粘性 + 30 min 硬过期）。若网关能看到 `conversation_id`，用它代替 user_id 做亲和 key，语义更准。
6. **`SchedulingContext` 字段膨胀**。审查后已敲定方案：拆出 `SchedulingRequest`（请求元数据 `user_id / color / order_type / session_id`）与 `SchedulingContext`（候选因子快照），避免 trait 参数失控（决策 D6 / E-D2）。
7. **OrderType × Affinity 优先级冲突**。审查后已敲定（决策 D7）：**OrderType 先过滤候选集，Affinity 在过滤后子集内生效**。例：Budget 客户的亲和渠道若已被价格上限过滤掉，Affinity 不应粘住贵渠道。
8. **Classifier 归属**。审查后已敲定（决策 E-D3）：`resolve_traffic_class` 落在 `burncloud-service-user`，由 Server 层注入 router，**保持 router crate 颜色无关**。
9. **PolicyPlane / PriceCache 重复**。审查后已敲定（决策 E-D4 / D11）：**复用现有 `crates/router/src/exchange_rate.rs` + `price_sync.rs`**，不新增独立 PolicyPlane / PriceCache 文件。
10. **DRY 风险**：`HealthProber` 与 `ChannelStateTracker` 职能重叠区需要在阶段 3 实施前划清，避免双向写入互相覆盖。

---

## Critical Files

| 路径 | 角色 | 阶段 |
|------|------|------|
| `crates/router/src/model_router.rs` | 调度入口；所有分层插入点 | MVP |
| `crates/router/src/scheduler/mod.rs` | `SchedulingRequest` / `SchedulingContext` 拆分；`build_context` 为 L4 保留 | MVP |
| `crates/router/src/scheduler/combined.rs` | L4 Scorer 实体，**保留不动** | — |
| `crates/router/src/aimd_limiter.rs` | 与 L2 Shaper 的 RPM 反哺接口（已从 `adaptive_limit.rs` 重命名为工业名 AIMD） | 阶段 2 |
| `crates/router/src/channel_state.rs` | L6 Prober 写入端、L3 Affinity 健康查询端 | MVP / 阶段 3 |
| `crates/router/src/circuit_breaker.rs` | 与 L3 Affinity 驱逐联动 | MVP |
| `crates/router/src/exchange_rate.rs` + `price_sync.rs` | 控制面异步刷新基础设施，复用为 PolicyPlane / PriceCache | 阶段 2+ |
| `crates/database/crates/channel/` | DB 字段扩展 | MVP（与阶段 2 合并） |
| `crates/database/crates/router-log/` | 可观测字段 | MVP（与阶段 3 字段合并） |
| `crates/service/crates/user/` | `resolve_traffic_class(user_id)` | MVP |

---

## 附录 A：术语对齐（工业名映射）

> **完整对照表已抽出**到 [`docs/code/GLOSSARY.md`](../code/GLOSSARY.md)，作为新成员入门 router 代码的速查文档。本节保留**优先落地的 10 个重命名**（按 ROI）作为路线图锚点。

### 优先落地的 10 个重命名（按 ROI）

| 现名 | 建议新名 | 落地策略 | 状态 |
|---|---|---|---|
| ~~`adaptive_limit.rs`~~ | `aimd_limiter.rs` | **最高 ROI 单点**——独立 PR 先行（审查决策 E-D8） | ✅ 已落地 |
| ~~`AdaptiveRateLimit`~~ | `AimdController` | 同上 PR 内同步 | ✅ 已落地 |
| ~~`AdaptiveSnapshot`~~ | `AimdSnapshot` | 同上 PR 内同步 | ✅ 已落地 |
| ~~`AdaptiveLimitConfig`~~ | `AimdConfig` | 同上 PR 内同步 | ✅ 已落地 |
| `ChannelStateTracker` | `AdjacencyTable` | 视第一个 PR review 体验决定是否继续 | 待评估 |
| `AffinityCache`（规划） | `FlowCache` | MVP 实施时直接采用新名 | 待 MVP |
| `ChannelBudget`（规划） | `TrTCMShaper` | 阶段 2 实施时直接采用新名 | 待阶段 2 |
| `TrafficColor` | `Dscp` | 视团队接受度 | 待评估 |
| `CombinedScheduler` | `WfqScheduler` | 视团队接受度 | 待评估 |
| `health_prober.rs`（规划） | `bfd.rs` | 阶段 3 解 BLOCK 后直接采用 | 待阶段 3 |
| Top-5 候选列表 | `FibBackupPaths / LfaList` | 文档术语统一即可 | 待评估 |
| `route_with_scheduler()` | `forward() / fib_lookup()` | 视团队接受度 | 待评估 |

---

## 附录 B：协议学习路径图（按阶段撬动的协议家族）

### 阶段 MVP + 阶段 2（单节点调度器）

**必读**：Token Bucket、GCRA、AIMD、Circuit Breaker、Exponential Backoff + Jitter、Rendezvous Hashing (HRW)
**有时间读**：Maglev Hashing、Power of Two Choices、WFQ / PQ、srTCM / trTCM (RFC 2697 / 2698)
**参考**：Netflix Hystrix 文档；Google Maglev 论文

### 阶段 3 + 阶段 3+（主动探测 + 指纹识别）

**必读**：BFD (RFC 5880)、DiffServ 架构 (RFC 2474)、DBSCAN 聚类
**有时间读**：ECN、DSCP 体系、Adjacency / Neighbor State 机、HMM 时序聚类

### 阶段 4（多实例分布式账本）

**必读**：Raft 论文、CRDT 综述、SWIM 论文
**有时间读**：HyParView、Lamport Timestamps / Vector Clocks、Quorum (W + R > N)
**参考**：《DDIA》第 5、9 章

### 阶段 5（BPP 联邦）

**必读**：BGP (RFC 4271，抓主干)、RPKI (RFC 6480)、mTLS、LFA / ECMP
**有时间读**：MPLS TE、BGP Communities、Route Reflector 架构
**参考**：Cloudflare BGP 博客系列

### 长期（若进入 P2P 市场形态）

**概念熟悉**：Kademlia DHT、Interledger Protocol、Lightning Network、FIX 协议、VCG 拍卖
**参考**：libp2p 文档

### 贯穿始终的心智模型（非具体协议）

- DiffServ / IntServ
- MPLS TE（预留 + 抢占 + 借用）
- SDN / OpenFlow（控制面数据面极致分离）
- 端到端原则（Saltzer / Reed / Clark 经典论文）
- QoS 四要素：Classify → Mark → Queue → Schedule

### 交易所 / 市场微观结构（仅作灵感来源，非必读）

> 当前 Burncloud 定位是"多供应商直连路由"，不是交易所，以下仅在你未来想借鉴"透明度"与"Order Type"之外的深层交易所机制时再看。

**偶尔翻翻**：

- **Hashflow RFQ** — Request-for-Quote 模式，与你"客户问价 → 供应商响应"的场景最接近，值得优先了解
- **《Trading and Exchanges》by Larry Harris** — 市场设计的第一本书，有空随便翻
- **LMAX Disruptor** — 低延迟消息 / 撮合架构，Rust 有对应实现，哪天路由热路径需要极致性能可参考

**暂不推荐深入**：Kyle / Glosten-Milgrom 定价模型、FIX 协议、NASDAQ ITCH / OUCH、Uniswap AMM、链上清算——这些属于"真做交易所"才需要，你的定位不需要。

### 周末 10 小时学习优先级

1. AIMD + Token Bucket + GCRA + Circuit Breaker + Exp Backoff（2 h）
2. Rendezvous Hash + Maglev + P2C（1 h）
3. SWIM 论文（1 h）
4. BGP 抓主干（Cloudflare 博客版，2 h）
5. Raft 论文原版（2 h）
6. 《DDIA》第 5、9 章（2 h）

---

## Verification Plan

- **阶段 MVP**：`cargo test -p burncloud-router affinity` 固定 user_id 粘性测试；手动构造 2 channel + 5 user 模拟场景，日志观察每用户 90% 以上请求落在同一 channel。
- **阶段 2**：wrk 压测 RPM 超限场景，router_logs 按 `rejected_by_shaper` 分组统计；sqlite / postgres 双后端跑迁移脚本。
- **阶段 3**（解 BLOCK 后）：停掉一个 channel 模拟宕机，30 s 内 prober 标记其 unhealthy；Grafana 按 `layer_decision` 画饼图。
- **阶段 4**（解 DEFER 后）：启 2 实例 + Redis，wrk 并发打满；Redis `GET rpm:channel_N` 与 channel 实际上游 429 率对比。

---

## Ready 审查衔接

本文档由 issue [#143](https://github.com/burncloud/burncloud/issues/143) ready 线程审查（CEO + Eng 双 Phase，共 23 决策、7 [TASTE DECISION]、1 [USER CHALLENGE]、13 Eng Findings）后沉淀。审查最终结论 **TASTE DECISION — 条件性 APPROVE，分三路径并行**：

| 路径 | 状态 | 内容 |
|------|------|------|
| 路径 1 | ✅ APPROVE 立即 | 文档合并（本文件）+ 附录 A 最高 ROI 重命名（`adaptive_limit.rs → aimd_limiter.rs`） |
| 路径 2 | ⚠️ CONDITIONAL APPROVE | MVP + 阶段 2，**前置**：(0) router_logs 数据验证 P1 痛点；(1) SQL 占位符迁移到 `ph/phs`；(2) `NoAvailableChannelsError` 改 `thiserror` |
| 路径 3 | ❌ REJECT 立即纳入 12 月路线图 | 阶段 3 / 3+ / 4 / 5 全部 DEFER |

**未澄清的 6 个关键问题**（merge 前作者需回应）：

1. **P1 证据**：能否用 router_logs 跑出"同 user_id 多 channel 分散"的实证数据？
2. **P4 客户分布**：当前付费客户有多少？OrderType 三分是从真实访谈还是假设？
3. **P6 BPP 可行性**：是否已与任一供应商接触过 BPP Exporter 概念？
4. **P7 ToS**：阶段 3 主动探测是否咨询过法务 / ToS？
5. **P10 带宽**：Burncloud 团队规模？六阶段 3–6 月是否挤占其他优先级？
6. **OrderType × Affinity 冲突解**：是否认同 OrderType 先过滤 → Affinity 子集生效的优先级？（本文档已按"是"沉淀，见 Part 2 / Part 4 取舍 7）

**关键 Dual Voices 缺口**：审查降级为单审查员模式，建议作者在实施前主动邀请第二视角对 P1 / P6 / P10 三个高风险未验证前提做对抗性盘查——这 3 条任何一条被证伪都会导致整个蓝图价值锐减。

---

## 维护说明

- 本文档是**活的蓝图（Living Reference）**：每次落地一个阶段后，回来更新对应小节的状态（DEFER → 已落地 / 已修订设计）。
- 术语表 `docs/code/GLOSSARY.md` 与本文档协同维护：本文档讲"为什么这样设计"，术语表讲"代码概念叫什么名字、对应哪个工业术语"。
- 若 ready 线程审查的 6 个未澄清问题有任何一条被新证据推翻，请在「Ready 审查衔接」一节补充更新，并在受影响小节加 `> **2026-MM-DD 更新**：` 注释。
