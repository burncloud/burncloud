# BurnCloud 路由错误契约 — 503 Reject 通道

> **范围**：BurnCloud LLM 路由（`crates/router`）在请求**未到达上游 LLM 提供商**之前主动拒绝时返回的 HTTP 503 响应契约。
> **蓝图来源**：`docs/design/channel-scheduler-hqos.md` § 阶段 MVP / 决策 D12。
> **目标读者**：客户端 / SDK 集成方、运维自动化、网关层重试逻辑。

---

## 为什么是 503 而不是 429

BurnCloud 路由层有 3 种"主动拒绝"路径：上游 LLM 提供商真正返回的 429 永远透传给客户端（带原始 `Retry-After` 等 header），而**路由器本地拒绝**（OrderType 价格约束 / 调度无可用渠道 / L2 Shaper 三色令牌桶）统一返回 **HTTP 503 Service Unavailable**——这样客户端可以从 status code 一眼区分：

| 场景 | Status | 来源 | 重试策略含义 |
|---|---|---|---|
| 上游 LLM 真正限速 | `429` | 上游透传 | 上游配额问题，等到 `Retry-After` |
| 路由本地拒绝（3 通道之一） | `503` | 路由器构造 | 本地容量 / 配置问题，按通道语义退避 |
| 上游 5xx | 上游 status 透传 | 上游透传 | 上游服务异常 |
| 上游网络全部失败 | `502 Bad Gateway` | 路由器构造 | 全部候选不可达 |

所有 503 拒绝响应**必带** `X-Rejected-By` 与 `Retry-After` 两个 header（蓝图决策 D12）。客户端必须读 `X-Rejected-By` 决定后续行为，仅看 status code 不够。

---

## 三个 reject 通道

### 1. `X-Rejected-By: order_type`

**触发条件**：客户端 token 配置了 OrderType 价格约束（如 `Budget { max_price_nanodollars: 1000 }`），但当前 group + model 下**所有候选渠道**的价格都超出该 cap。

**典型成因**：
- 用户买的是预算套餐但选了高价模型
- 渠道价格刚好上调把所有候选挤出 cap
- 渠道的 `pricing_region` 改了但客户端 cap 没跟进

**`Retry-After` 语义**：固定 `60` 秒。**重试策略**：单纯重试**不会**让 cap 重新匹配——客户端应当：
- 升级到更高 OrderType（如从 `budget` 切到 `value`）；或
- 切换到更便宜的模型；或
- 联系运维上调 cap

不要做指数退避——配置不变情况下，60 秒后第二次请求仍会被同样拒。

**示例响应体**：
```json
{
  "error": {
    "message": "OrderType budget filtered out all candidates (no channel meets price constraint)",
    "type": "service_unavailable",
    "code": "no_available_channels",
    "rejected_by": "order_type"
  }
}
```

---

### 2. `X-Rejected-By: scheduler`

**触发条件**：`route_with_scheduler` 返回 `NoAvailableChannelsError` 中**非** OrderType 子串的错误。最典型情况：当前 group + model 下所有候选渠道在 `channel_state_tracker` 视为不可用（认证失败、上游限速、熔断、余额耗尽）。

**典型成因**：
- 全部渠道都熔断中（最近 5 次连续失败）
- 全部渠道认证 token 过期 / 错误
- 全部渠道余额标记为 Exhausted
- 模型/group 配置错误（无对应 `channel_abilities` 行）

**`Retry-After` 语义**：固定 `60` 秒。**重试策略**：60 秒覆盖的场景：
- 熔断器默认冷却 30 秒（蓝图 `CIRCUIT_BREAKER_COOLDOWN_SECS`），60 秒后大概率有渠道回到 Closed 状态
- AIMD limiter 单条渠道恢复

客户端建议**线性退避** 60 秒第一次重试 + 指数退避后续重试（120s / 240s / 阻断告警），避免雪崩。

**示例响应体**：
```json
{
  "error": {
    "message": "No available channels for model 'gpt-4o': All channels are currently unavailable (rate limited, auth failed, or exhausted)",
    "type": "service_unavailable",
    "code": "no_available_channels",
    "rejected_by": "scheduler"
  }
}
```

---

### 3. `X-Rejected-By: shaper`

**触发条件**：route_with_scheduler 已经返回了非空候选列表，但 failover 循环里**每一个**候选都被 L2 Shaper（`crates/router/src/rate_budget.rs::InMemoryBudget`）的三色令牌桶 `try_consume` 主动拒绝（`ConsumeOutcome::Rejected`）——即所有候选渠道在当前一分钟窗口内 RPM/TPM 已耗尽，且无法跨颜色借用。

**典型成因**：
- 单分钟内某个 group 的请求量大幅 burst，桶已耗尽
- 渠道 `rpm_cap` 配置过低
- DiffServ 颜色优先级冲突（如 Yellow 用户被 Green 用户挤出）

**`Retry-After` 语义**：固定 `60` 秒。**重试策略**：**60 秒后桶必然 refill**（`REFILL_WINDOW = Duration::from_secs(60)`，`crates/router/src/rate_budget.rs:250`），因此客户端按 60 秒精确退避即可恢复。建议：
- 第一次重试等待 `Retry-After` + 抖动（例如 60 ± 10s 的随机化避免 thundering herd）
- 后续重试指数退避（120s / 240s）防止过载

**示例响应体**：
```json
{
  "error": {
    "message": "All candidate channels rejected by rate budget shaper",
    "type": "service_unavailable",
    "code": "rate_budget_exhausted",
    "rejected_by": "shaper"
  }
}
```

注意 `code` 字段值 `rate_budget_exhausted` 与前两个通道的 `no_available_channels` **不同**——客户端可以同时基于 `code` 与 `rejected_by` 做精细化分支。

---

## 公共 Header 契约

所有 503 拒绝响应必带以下 header：

| Header | 值 | 含义 |
|---|---|---|
| `X-Rejected-By` | `order_type` / `scheduler` / `shaper` | 哪个路由层做出了拒绝决定 |
| `Retry-After` | `60`（当前固定） | 客户端建议退避秒数 |
| `Content-Type` | `application/json` | 响应体是 JSON |

> **未来扩展**：`Retry-After` 当前硬编码 60 秒。phase 2.5 可能让它依据 shaper 桶 refill 剩余时间动态计算。客户端**应**按 header 实际值退避，不要硬编码 60。

---

## 客户端退避策略矩阵

总结三个通道的差异：

| 通道 | 重试有意义？ | 建议策略 | 何时升级人工干预 |
|---|---|---|---|
| `order_type` | ❌（配置问题） | 不重试，直接报错给上层；提示用户升级套餐 / 切换模型 | 立即——同样的请求重试不会变化 |
| `scheduler` | ✅ | 60s 线性退避 1 次 → 120s/240s 指数退避 → 告警 | 3 次失败后告警 |
| `shaper` | ✅ | 60s + 抖动 → 120s/240s 指数退避 → 告警 | 5 次失败后告警（短时 burst 通常自愈） |

---

## 与 router_logs 的关联

每个 503 拒绝响应在 `router_logs` 表都会有对应行（migration 0011 加的 `layer_decision` 列）：

| `X-Rejected-By` 值 | `router_logs.layer_decision` 值 |
|---|---|
| `order_type` | `None`（暂未填，归属 phase 2.5 完善） |
| `scheduler` | `None`（同上） |
| `shaper`（全候选拒）| `shaper_reject` |
| 上游成功（与 503 无关） | `shaper_own` / `shaper_borrow` / `shaper_unconfigured` |

`router_logs.traffic_color` 同时记录 L1 Classifier 给请求分配的 `G` / `Y` / `R`，可用于追溯：高优先级 Green 用户为何被 shaper 拒？答案通常是预留比例配置错误。

---

## 路由日志可观测性

实时观测 503 拒绝率：

```sql
-- 最近 1 小时各通道拒绝率
SELECT
  layer_decision,
  COUNT(*) AS rejects,
  ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (), 2) AS pct
FROM router_logs
WHERE status_code = 503
  AND created_at > strftime('%s', 'now') - 3600
GROUP BY layer_decision;
```

`/console/internal/health` endpoint 实时 JSON 暴露：
- `budget_snapshots[]`：每个已配置 cap 的渠道当前剩余三色 RPM/TPM
- `fail_open_count`：累计未配置渠道的 fail-open 通过次数（FM2 silent failure 可见性）

---

## 相关引用

- 蓝图决策 D12：`docs/design/channel-scheduler-hqos.md` § 阶段 MVP "HTTP 契约（审查决策 D12）"
- 实现：
  - `order_type` / `scheduler`：`crates/router/src/lib.rs` proxy_logic 内 `route_with_scheduler` 错误分支（issue #145）
  - `shaper`：`crates/router/src/lib.rs` proxy_logic 失败回退循环之后的"全候选被 shaper 拒"分支（issue #151）
- 对应 issue：#145（rate_budget 模块）/ #150（L1 Classifier 接入）/ #151（L2 Shaper 接入热路径）
