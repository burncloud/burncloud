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
    - [x] `server`: 后台任务从队列消费日志并批量写入 `logs` 表 (SQLite)。(Implemented in Router process directly for now)

- [x] **Task 5.2: Token Counting**
    - [x] `router`: 初步实现 Basic Token Estimation (`len/4`)。
    - [ ] `router`: 集成 `tiktoken` (Future Work for higher precision).
    - [ ] `service`: 扣除用户余额 (To be implemented in Service layer).

---
*Updated by AI Agent - LiveView Strategy*
