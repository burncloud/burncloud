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
- [ ] **Task 5.1: Async Logging**
    - [ ] `router`: 使用 `tokio::mpsc` 将请求日志发送到异步队列。
    - [ ] `server`: 后台任务从队列消费日志并批量写入 `logs` 表 (SQLite/ClickHouse)。

- [ ] **Task 5.2: Token Counting**
    - [ ] `router`: 集成 `tiktoken` (或 Rust 等价库) 计算 Prompt Token。
    - [ ] `router`: 对于流式响应，估算或累加 Completion Token。
    - [ ] `service`: 扣除用户余额。

---
*Updated by AI Agent - LiveView Strategy*
