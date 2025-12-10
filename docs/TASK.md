# BurnCloud 开发任务清单 (Task List)

> 本文档基于 `docs/ARCHITECTURE_EVOLUTION.md` 拆解，遵循 **原子化开发 (Atomic Development)** 原则。
> 状态标记: [ ] Pending, [/] In Progress, [x] Completed
> **注意**: 已完成的任务 (Phase 1-13) 已归档至 `docs/TASK_ARCHIVE.md`。

---

## 📅 Phase 14: 分布式与企业级架构 (Distributed & Enterprise)
- [x] **Task 14.1: PostgreSQL Support**
    - [x] `database`: 引入 `sqlx-postgres`，支持可选的 PG 后端，用于海量日志存储和复杂的计费查询。
    - [x] `database`: 抽象数据库接口，支持 SQLite/Postgres 切换 (via `AnyPool`).
- [x] **Task 14.2: Redis Integration**
    - [x] `common`: 引入 `redis` crate。
    - [x] `service-redis`: 创建 RedisService 封装。
    - [ ] `router`: 将限流 (Rate Limiter) 和 令牌验证 (Token Validation) 迁移至 Redis (可选)。
- [x] **Task 14.3: User Management & RBAC**
    - [x] `database`: 设计用户角色表 (Role-Based Access Control)。
    - [x] `database`: 实现用户注册、角色分配方法。
    - [x] `server`: 实现用户注册、登录 API (GitHub/OIDC)。
- [x] **Task 14.4: Unified Protocol Adaptors (v0.3)**
    - [x] `router`: 实现 `GeminiAdaptor` (OpenAI Request -> Gemini API -> OpenAI Response)。
    - [x] `router`: 实现 `ClaudeAdaptor` (OpenAI Request -> Anthropic API -> OpenAI Response)。
    - [x] `router`: 更新 `proxy_logic` 以支持基于 `Upstream` 配置的自动协议转换。

---

## 📅 Phase 15: 核心重构与系统点火 (Core Refactor & System Ignition)
> 目标: 抛弃脆弱的强类型绑定，建立“泛型透传”机制；实现基于 Ability 的路由引擎，打通 Client 到 Upstream 的全链路。

- [x] **Task 15.1: Router 重构 - 泛型透传 (Generic Passthrough)**
    - [x] `common`: 定义 `GenericRequest` 结构体，只保留 `model`, `messages`, `stream` 为强类型，其余字段使用 `HashMap<String, serde_json::Value>` 透传。
    - [x] `router`: 修改核心转发逻辑，不再试图解析所有参数，确保上游新参数（如 Google `thinking`）能无缝通过。
    - [ ] `router`: 引入 `rhai` 或 `mlua` (可选) 为未来处理复杂参数映射做准备。

- [x] **Task 15.2: Ability 路由引擎 (The Ability Engine)**
    - [x] `database`: 设计 `abilities` 表结构 (Group + Model + ChannelId)，用于扁平化快速查询。
    - [x] `router`: 实现基于 Ability 的路由查找算法 (Priority -> Weight -> Random)。
    - [x] `router`: 实现 `Group` 逻辑，确保用户只能访问其权限组内的模型。

- [ ] **Task 15.3: 通用适配器与协议降级 (Generic Adaptor)**
    - [ ] `router`: 创建 `UniversalAdaptor`，支持通过配置定义 Header/Body 的覆写 (Override)。
    - [ ] `router`: 确保在无法识别特定协议参数时，能够安全降级并透传原始 JSON。

- [ ] **Task 15.4: 全链路点火 (End-to-End Ignition)**
    - [ ] `server`: 将 API Gateway (Axum/Gin) 与新的 Router 逻辑完全打通。
    - [ ] `database`: 初始化测试用的 `channels` (如 OpenAI, Gemini) 和 `models` 数据。
    - [ ] `client`: 验证聊天界面 (Chat UI) 能成功发起请求并接收流式响应。

---
*Updated by AI Agent - LiveView Strategy*
