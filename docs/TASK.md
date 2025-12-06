# BurnCloud 开发任务清单 (Task List)

> 本文档基于 `docs/ARCHITECTURE_EVOLUTION.md` 拆解，遵循 **原子化开发 (Atomic Development)** 原则。
> 状态标记: [ ] Pending, [/] In Progress, [x] Completed

---

## 📅 Phase 1-4 (Completed)
- [x] 国产模型支持 (DeepSeek/Qwen)
- [x] 协议适配器 (Gemini/Claude)
- [x] 负载均衡与故障转移
- [x] 控制面 API 骨架

---

## 📅 Phase 5: 精确计费与日志 (Billing & Logging)
- [x] **Task 5.1: Async Logging**
    - [x] `router`: 使用 `tokio::mpsc` 将请求日志发送到异步队列。
    - [x] `router`: 后台任务消费日志并批量写入 `logs` 表 (SQLite)。

- [x] **Task 5.2: Token Counting & Quota**
    - [x] `router`: 初步实现 Basic Token Estimation (`len/4`)。
    - [x] `router`: 实现 Quota 检查 (Pre-check) 与 扣费 (Async Update)。
    - [x] `server`: 实现 `/api/logs` 和 `/api/usage` 接口。

## 📅 Phase 6: 前端仪表盘集成 (Dashboard Integration)
- [x] **Task 6.1: Dashboard UI**
    - [x] `client-dashboard`: 连接 `/console/logs` 展示调用日志。
    - [x] `client-dashboard`: 展示用户余额与消耗统计。
- [x] **Task 6.2: Channel Management UI**
    - [x] `client-settings`: 连接 `/console/channels` 实现渠道 CRUD。
- [x] **Task 6.3: Token Management UI**
    - [x] `client-settings`: 连接 `/console/tokens` 实现令牌管理。

## 📅 Phase 7: 高级路由与分组 (Advanced Routing) - Completed
- [x] **Task 7.1: Group Management API**
    - [x] `server`: 完善 `/console/groups` API (CRUD & Member assignment).
    - [x] `database`: 确保 `router_groups` 关联查询性能.
- [x] **Task 7.2: Group Management UI**
    - [x] `client-settings`: 实现分组管理界面 (创建分组、分配渠道权重).
- [x] **Task 7.3: Router Group Logic**
    - [x] `router`: 验证基于 Group 的路由分发策略 (RoundRobin/Weighted).

## 📅 Phase 8: 统一网关 (Unified Gateway) - Completed
- [x] **Task 8.1: Router Library-fication**
    - [x] `router`: 重构为 Axum Library (`create_router_app`).
- [x] **Task 8.2: Path Normalization**
    - [x] `server`: 迁移管理 API 至 `/console/api/*`.
    - [x] `client`: 更新前端 API 调用路径.
- [x] **Task 8.3: Gateway Integration**
    - [x] `server`: 集成 Router 作为 Fallback Service.
    - [x] `main`: 统一入口至 3000 端口.

## 📅 Phase 9: 高可用与限流 (Robustness & Rate Limiting)
- [ ] **Task 9.1: Rate Limiter Middleware**
    - [ ] `router`: 实现基于 Token 的限流 (TokenBucket/LeakyBucket).
    - [ ] `database`: Redis 集成准备 (可选，先基于内存).
- [ ] **Task 9.2: Circuit Breaker (熔断器)**
    - [ ] `router`: 自动检测上游连续失败并暂时剔除.
    - [ ] `server`: 渠道健康状态监控 API.



---
*Updated by AI Agent - LiveView Strategy*
