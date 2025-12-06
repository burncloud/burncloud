# BurnCloud 开发任务清单 (Task List)

> 本文档基于 `docs/ARCHITECTURE_EVOLUTION.md` 拆解，遵循 **原子化开发 (Atomic Development)** 原则。
> 状态标记: [ ] Pending, [/] In Progress, [x] Completed

---

## 📅 Phase 1: 国产模型支持与基础路由增强 (Domestic Models & Basic Routing)
**目标**: 解决国内用户痛点，支持 DeepSeek、Qwen 等模型，并确保路由层的稳定性。

- [x] **Task 1.1: DeepSeek Support**
    - [x] `router`: 在 `AuthType` 中添加 `DeepSeek` 枚举。
    - [x] `router`: 实现 Bearer Token 注入逻辑 (类似 OpenAI)。
    - [x] `test`: 编写 `test_deepseek_proxy` 集成测试 (Mock)。

- [x] **Task 1.2: Qwen (通义千问) Support**
    - [x] `router`: 在 `AuthType` 中添加 `Qwen` (阿里云 DashScope) 枚举。
    - [x] `router`: 实现 `Authorization: Bearer <API-KEY>` 注入 (注意: 阿里云有时也用 `X-DashScope-WorkSpace`，需确认标准)。
    - [x] `test`: 编写 `test_qwen_proxy` 集成测试。

- [x] **Task 1.3: Router Config Hot Reload**
    - [x] `router`: 实现配置热加载机制 (当数据库更新 Upstream 时，Router 无需重启)。
    - [x] `server`: 提供 `/api/internal/reload` 接口或基于 File Watcher/DB Polling。

---

## 📅 Phase 2: 协议适配器 (Protocol Adaptors)
**目标**: 实现“万物转 OpenAI”，这是对标 OneAPI 的核心能力。

- [x] **Task 2.1: Gemini to OpenAI Adaptor**
    - [x] `router/adaptor`: 创建 `GeminiAdaptor` 结构体。
    - [x] `router`: 实现 Request 转换: `OpenAI ChatCompletion` -> `Gemini generateContent`。
    - [x] `router`: 实现 Response 转换: `Gemini JSON` -> `OpenAI JSON`。
    - [x] `test`: 真实调用 Gemini API，客户端使用 OpenAI SDK 接收。

- [x] **Task 2.2: Claude to OpenAI Adaptor**
    - [x] `router/adaptor`: 创建 `ClaudeAdaptor` 结构体。
    - [x] `router`: 实现 Request/Response/Stream 转换。

---

## 📅 Phase 3: 智能负载均衡 (Smart Load Balancing)
**目标**: 提高可用性，支持多渠道并发与故障转移。

- [x] **Task 3.1: Upstream Grouping**
    - [x] `database`: 修改 Schema，引入 `ChannelGroup` 或 `ModelMapping` 表。
    - [x] `router`: 逻辑修改，从“匹配路径找一个 Upstream”变为“匹配模型名找一组 Upstream”。

- [x] **Task 3.2: Load Balancing Strategies**
    - [x] `router/balancer`: 实现 `RoundRobin` (轮询) 策略。
    - [x] `router/balancer`: 实现 `Weighted` (权重) 策略。

- [x] **Task 3.3: Failover Mechanism**
    - [x] `router`: 实现重试逻辑。当 Upstream 返回 5xx 或超时，自动重试组内下一个 Upstream。
    - [x] `service`: 记录渠道健康状态 (Healthy/Dead)。

---

## 📅 Phase 4: 运营级控制面 (Control Plane)
**目标**: 提供完整的管理 API 和 UI。

- [x] **Task 4.1: Channel Management API**
    - [x] `server`: 实现 `POST /api/channels` (增), `GET` (查), `PUT` (改), `DELETE` (删)。
    - [x] `service`: 封装 `ChannelService`。

- [x] **Task 4.2: Token Management API**
    - [x] `server`: 实现 `POST /api/tokens` (创建兑换码/访问令牌)。
    - [x] `database`: 完善 `tokens` 表 (余额、过期时间、无限额度标记)。

- [ ] **Task 4.3: Frontend Integration & Console Prefix**
    - [ ] **Subtask 4.3.1: API Route Refactoring**
        - [ ] `server`: 将所有管理 API (Channel/Group/Token) 移动到 `/console` 前缀下，避免与 `/api/v1/...` (LLM请求) 冲突。
        - [ ] `server`: 确保 `/api` 前缀预留给未来的业务逻辑或保持兼容。
    - [ ] **Subtask 4.3.2: Frontend API Client**
        - [ ] `client/shared`: 封装 `ApiClient`，配置 Base URL 为 `http://localhost:4000/console`。
        - [ ] `client`: 实现 HTTP 请求方法 (GET, POST, DELETE)。
    - [ ] **Subtask 4.3.3: Channel Management UI**
        - [ ] `client/api`: 使用 `ApiClient` 获取真实 Channel 列表。
        - [ ] `client/api`: 实现“创建渠道”表单。

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
*Updated by AI Agent*
