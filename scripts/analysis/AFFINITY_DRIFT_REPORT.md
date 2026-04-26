# 亲和漂移（Affinity Drift）实证分析报告

> **目的**：验证 issue [#143](https://github.com/burncloud/burncloud/issues/143) 宣称的「5 用户 2 渠道摇摆」痛点是否真实存在并量化，作为 MVP 调度器分层改造（亲和层 + OrderType + 染色）的**前置门槛**。
> **来源决策**：审查 D3 / 路径 2 前置条件 0（CONDITIONAL APPROVE 的核心阻塞项）。
> **跑分脚本**：[`scripts/analysis/affinity_drift.sql`](./affinity_drift.sql)
> **关联文档**：[`docs/design/channel-scheduler-hqos.md`](../../docs/design/channel-scheduler-hqos.md) Part 4 取舍 7

---

## 1. 方法论

### 1.1 数据范围
- 表：`router_logs`
- 时间窗口：**最近 7 天**（如需调整，改 SQL 文件中所有 `WINDOW = 7` 行）
- 过滤条件：
  - `status_code` ∈ [200, 299]（仅成功请求；4xx/5xx 不参与亲和判定）
  - `user_id IS NOT NULL`（排除匿名/系统调用）
  - `upstream_id IS NOT NULL`（排除尚未路由的请求）
  - `model IS NOT NULL`（亲和分析必须按 model 维度分组——同 user 跨 model 走不同 channel 是正常路由，不算摇摆）

### 1.2 字段映射
| 报告 / Issue 用语 | router_logs 实际列 | 说明 |
|------------------|-------------------|------|
| `channel_id` | `upstream_id` | 表里叫 upstream_id，是 channel_providers.id 的字符串形式 |
| 用户 | `user_id` | TEXT，应用层 user 标识 |
| 模型 | `model` | TEXT，亲和必须按 model 维度 |
| 请求时间 | `created_at` | sqlite TEXT / postgres TIMESTAMP |

### 1.3 核心指标

| 指标 | 含义 | 计算方式 |
|-----|------|---------|
| **channels_used** | 每个 (user, model) 对在窗口期使用了几个不同 channel | `COUNT(DISTINCT upstream_id)` |
| **dominant_share** | 主导 channel 在该 (user, model) 总命中中的占比 | `MAX(hits) / SUM(hits)` |
| **HHI** | Herfindahl-Hirschman 集中度指数 | `SUM(channel_share²)` |
| **MVP 命中率** | dominant_share ≥ 0.9 的 (user, model) 对占比 | issue MVP 验证目标的"10 次中 ≥ 9 次落同 channel" |

### 1.4 样本最小阈值
- 每 (user, model) 对至少 5 次请求才纳入 dominant_share / HHI 统计（Q4、Q5、Q7）
- Q6 摇摆清单要求至少 10 次请求（避免单次随机噪音被当成"摇摆"）
- 全样本 distinct_users ≥ 10、total_requests ≥ 1000 才视为有效分析（Q1 校验）

---

## 2. 决策门槛（go / no-go MVP）

跑完 SQL 后用此表对照 → 判断是否启动 MVP 实施。

| Q | 指标 | 强支持 MVP（GO） | 不支持 MVP（NO-GO） | 灰区（需复审） |
|---|-----|----------------|-------------------|--------------|
| Q1 | distinct_users / total_requests | ≥ 10 用户、≥ 1000 请求 | < 5 用户或 < 200 请求 → 样本量不足，先收集数据 | 之间 |
| Q3 | channels_used ≥ 2 的 (user,model) 对占比 | **> 30%** | < 10% | 10–30% |
| Q4 | P50 dominant_share | **< 0.7** | > 0.95 | 0.7–0.95 |
| Q4 | pct_meeting_mvp_target（≥ 0.9 的占比） | **< 60%** | > 90% | 60–90% |
| Q4 | pairs_severely_split（< 0.5）数量 | **≥ 5 个 (user,model) 对** | 0 个 | 1–4 个 |
| Q5 | mean_hhi | **< 0.7** | > 0.9 | 0.7–0.9 |
| Q7 | recent_7d vs prior_7d 趋势 | recent 比 prior 显著恶化（mean_share 降 ≥ 0.05） | 稳定或好转 | 微小波动 |

**判定逻辑**：
- 若 ≥ 3 项指标落 **GO** 且 0 项落 NO-GO → **APPROVE MVP 启动**
- 若 ≥ 2 项落 **NO-GO** → **拒绝 MVP**，issue P1 痛点假设被证伪，转「方案 D」customer discovery
- 其他情况 → **灰区**，需要团队人工评审 + 可能扩窗口到 30 天再跑一次

---

## 3. 待填结果（运行 SQL 后填入）

> 跑 `scripts/analysis/affinity_drift.sql` 后把数字粘到下表。空格用 `—` 填占位。

### 3.1 Q1 基础规模

| 指标 | 实测值 | 阈值 | 通过？ |
|-----|-------|------|-------|
| total_requests | — | ≥ 1000 | ☐ |
| distinct_users | — | ≥ 10 | ☐ |
| distinct_upstreams | — | ≥ 2 | ☐ |
| distinct_models | — | ≥ 1 | ☐ |
| window_start ~ window_end | — ~ — | — | — |

**样本量结论**：☐ 充分 / ☐ 不足（先扩窗口或收集更多数据）

### 3.2 Q3 摇摆广度（channels_used 直方图）

| channels_used | user_model_pairs | pct_of_total |
|--------------|------------------|--------------|
| 1（完美亲和） | — | — % |
| 2（轻度摇摆） | — | — % |
| 3 | — | — % |
| 4+ | — | — % |
| **≥ 2 合计** | — | **— %** |

**Q3 判定**：☐ GO（≥ 30%） / ☐ NO-GO（< 10%） / ☐ 灰区

### 3.3 Q4 摇摆深度（dominant_share 分位数）

| 指标 | 实测值 | 解读 |
|-----|-------|------|
| analyzed_pairs | — | 通过样本阈值的 (user,model) 对数 |
| mean_share | — | 平均主导渠道占比 |
| p25_share | — | 25% 的 (user,model) 主导占比 ≤ 此值 |
| p50_share | — | 中位数 |
| p75_share | — | — |
| p90_share | — | （仅 PG 版本可算） |
| pairs_meeting_mvp_target | — | dominant_share ≥ 0.9 的对数 |
| **pct_meeting_mvp_target** | **— %** | issue MVP 目标当前的"自然命中率" |
| pairs_severely_split | — | dominant_share < 0.5 的对数（典型摇摆） |

**Q4 判定**：☐ GO / ☐ NO-GO / ☐ 灰区
（GO 条件：P50 < 0.7 **或** pct_meeting_mvp_target < 60% **或** severely_split ≥ 5）

### 3.4 Q5 HHI 集中度

| 指标 | 实测值 |
|-----|-------|
| analyzed_pairs | — |
| mean_hhi | — |
| pairs_perfectly_affine（HHI = 1.0） | — |
| pairs_mostly_affine（0.8 ≤ HHI < 1.0） | — |
| pairs_drifting（0.5 ≤ HHI < 0.8） | — |
| pairs_severely_drifting（HHI < 0.5） | — |

**Q5 判定**：☐ GO（mean_hhi < 0.7） / ☐ NO-GO（> 0.9） / ☐ 灰区

### 3.5 Q6 摇摆用户实例（"5 用户 2 渠道"实证）

> 粘 Q6 输出的前 10 行（已按 total_hits 降序）。

```
user_id    | model       | channels_used | total_hits | channel_distribution
-----------+-------------+---------------+------------+----------------------
—          | —           | —             | —          | —
（粘贴此处）
```

**P1 实证结论**：☐ 找到 ≥ 5 个 channels_used = 2 的 (user, model) 对（吻合 issue 描述场景）
            / ☐ 未找到（issue 描述场景在生产中未出现）

### 3.6 Q7 趋势对比（可选）

| window_label | analyzed_pairs | mean_share | pairs_severely_split |
|-------------|----------------|------------|---------------------|
| recent_7d | — | — | — |
| prior_7d | — | — | — |

**Q7 判定**：☐ 恶化中（recent mean_share 比 prior 低 ≥ 0.05） / ☐ 稳定 / ☐ 好转

---

## 4. 综合决策

> 填完上面表格后，在此写最终结论。

**GO 项数**：— / 6
**NO-GO 项数**：— / 6
**灰区项数**：— / 6

**最终判定**：
- ☐ ✅ **APPROVE MVP**（GO ≥ 3 且 NO-GO = 0）：解除 D3 阻塞条件，可启动子任务 6（MVP 调度器分层改造）。
- ☐ ❌ **REJECT MVP**（NO-GO ≥ 2）：P1 痛点假设被证伪，建议转「方案 D」——先做 customer discovery（访谈 3-5 个客户），再决定是否需要亲和层。
- ☐ ⚠️ **灰区 — 复审**：在 issue #143 评论中召集团队 + 可能扩窗口到 30 天再跑一次。

**填表人** / **填表日期**：— / —

---

## 5. 数据样本不足时的备选路径

如果 Q1 校验未通过（total_requests < 1000 或 distinct_users < 10）：

1. **短期（1–2 周）**：保持 router_logs 写入，每周复跑一次本分析
2. **中期**：检查是否有日志保留策略导致历史数据被清掉（看 `router_logs` 实际行数 vs 预期）
3. **长期**：考虑是否亲和痛点本身在低流量阶段不显著——MVP 优先级可能应该让位于其他更紧迫问题

---

## 6. 已知局限

1. **没有 conversation_id**：当前 router_logs 只有 user_id，无法区分"同一用户的不同对话"。如果 issue 假设的痛点其实是按 conversation 维度，本分析会**低估**摇摆程度（同 user 的不同对话路由到不同 channel 是合理的）。
2. **upstream_id 是字符串**：分析时按字符串相等判断 channel；如果 channel_providers 重新分配过 ID（drop + recreate），同一 channel 可能被算成两个。
3. **status_code = 2xx 过滤**：忽略了"主 channel 返回 5xx 触发 failover 切到备用 channel"这种良性摇摆——这种摇摆是 failover 设计的预期行为，亲和层不应该消除它。本分析可能**高估**了"病理性摇摆"。
4. **窗口内 channel 集合变化**：如果某 channel 在窗口期内被禁用/重启，路由系统的"摇摆"实际是"渠道集合变化"导致的，不是调度器问题。Q6 实例需要人工 sanity check 排除这类。

---

## 7. 维护说明

- 跑完一次后保留填好的报告：`cp AFFINITY_DRIFT_REPORT.md AFFINITY_DRIFT_REPORT_2026MMDD.md`，签 git
- 若决策为 REJECT，在 docs/design/channel-scheduler-hqos.md 末尾加注释说明 P1 被证伪
- 若决策为 APPROVE，在子任务 6（MVP 实施）的 PR description 引用本报告版本作为前置条件已满足的证据
