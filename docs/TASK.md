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
- [/] **Task 6.1: Dashboard UI**
    - [x] `client-dashboard`: 连接 `/console/logs` 展示调用日志。
    - [ ] `client-dashboard`: 展示用户余额与消耗统计。
- [ ] **Task 6.2: Channel Management UI**
    - [ ] `client-settings`: 连接 `/console/channels` 实现渠道 CRUD。
- [ ] **Task 6.3: Token Management UI**
    - [ ] `client-settings`: 连接 `/console/tokens` 实现令牌管理。


---
*Updated by AI Agent - LiveView Strategy*
