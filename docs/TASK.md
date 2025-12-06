# BurnCloud 开发任务清单 (Task List)

> 本文档基于 `docs/ARCHITECTURE_EVOLUTION.md` 拆解，遵循 **原子化开发 (Atomic Development)** 原则。
> 状态标记: [ ] Pending, [/] In Progress, [x] Completed

---

## 📅 Phase 1: 国产模型支持与基础路由增强 (Domestic Models & Basic Routing)
**目标**: 解决国内用户痛点，支持 DeepSeek、Qwen 等模型，并确保路由层的稳定性。

- [ ] **Task 1.1: DeepSeek Support**
    - [ ] `router`: 在 `AuthType` 中添加 `DeepSeek` 枚举。
    - [ ] `router`: 实现 Bearer Token 注入逻辑 (类似 OpenAI)。
    - [ ] `test`: 编写 `test_deepseek_proxy` 集成测试 (Mock)。

- [ ] **Task 1.2: Qwen (通义千问) Support**
    - [ ] `router`: 在 `AuthType` 中添加 `Qwen` (阿里云 DashScope) 枚举。
    - [ ] `router`: 实现 `Authorization: Bearer <API-KEY>` 注入 (注意: 阿里云有时也用 `X-DashScope-WorkSpace`，需确认标准)。
    - [ ] `test`: 编写 `test_qwen_proxy` 集成测试。

- [ ] **Task 1.3: Router Config Hot Reload**
    - [ ] `router`: 实现配置热加载机制 (当数据库更新 Upstream 时，Router 无需重启)。
    - [ ] `server`: 提供 `/api/internal/reload` 接口或基于 File Watcher/DB Polling。

---

## 📅 Phase 2: 协议适配器 (Protocol Adaptors)
**目标**: 实现“万物转 OpenAI”，这是对标 OneAPI 的核心能力。

- [ ] **Task 2.1: Gemini to OpenAI Adaptor**
    - [ ] `router/adaptor`: 创建 `GeminiAdaptor` 结构体。
    - [ ] `router`: 实现 Request 转换: `OpenAI ChatCompletion` -> `Gemini generateContent`。
    - [ ] `router`: 实现 Response 转换: `Gemini JSON` -> `OpenAI JSON`。
    - [ ] `router`: **难点**: 实现 Streaming Response 转换 (SSE 格式转换)。
    - [ ] `test`: 真实调用 Gemini API，客户端使用 OpenAI SDK 接收。

- [ ] **Task 2.2: Claude to OpenAI Adaptor**
    - [ ] `router/adaptor`: 创建 `ClaudeAdaptor` 结构体。
    - [ ] `router`: 实现 Request/Response/Stream 转换。

---

## 📅 Phase 3: 智能负载均衡 (Smart Load Balancing)
**目标**: 提高可用性，支持多渠道并发与故障转移。

- [ ] **Task 3.1: Upstream Grouping**
    - [ ] `database`: 修改 Schema，引入 `ChannelGroup` 或 `ModelMapping` 表。
    - [ ] `router`: 逻辑修改，从“匹配路径找一个 Upstream”变为“匹配模型名找一组 Upstream”。

- [ ] **Task 3.2: Load Balancing Strategies**
    - [ ] `router/balancer`: 实现 `RoundRobin` (轮询) 策略。
    - [ ] `router/balancer`: 实现 `Weighted` (权重) 策略。

- [ ] **Task 3.3: Failover Mechanism**
    - [ ] `router`: 实现重试逻辑。当 Upstream 返回 5xx 或超时，自动重试组内下一个 Upstream。
    - [ ] `service`: 记录渠道健康状态 (Healthy/Dead)。

---

## 📅 Phase 4: 运营级控制面 (Control Plane)
**目标**: 提供完整的管理 API 和 UI。

- [ ] **Task 4.1: Channel Management API**
    - [ ] `server`: 实现 `POST /api/channels` (增), `GET` (查), `PUT` (改), `DELETE` (删)。
    - [ ] `service`: 封装 `ChannelService`。

- [ ] **Task 4.2: Token Management API**
    - [ ] `server`: 实现 `POST /api/tokens` (创建兑换码/访问令牌)。
    - [ ] `database`: 完善 `tokens` 表 (余额、过期时间、无限额度标记)。

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
*Created by AI Agent based on docs/ARCHITECTURE_EVOLUTION.md*