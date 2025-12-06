# BurnCloud 开发任务清单 (Task List)

> 本文档基于 `docs/ARCHITECTURE_EVOLUTION.md` 拆解，遵循 **原子化开发 (Atomic Development)** 原则。
> 状态标记: [ ] Pending, [/] In Progress, [x] Completed
> **注意**: 已完成的任务 (Phase 1-13) 已归档至 `docs/TASK_ARCHIVE.md`。

---

## 📅 Phase 14: 分布式与企业级架构 (Distributed & Enterprise)
- [ ] **Task 14.1: PostgreSQL Support**
    - [ ] `database`: 引入 `sqlx-postgres`，支持可选的 PG 后端，用于海量日志存储和复杂的计费查询。
    - [ ] `database`: 抽象数据库接口，支持 SQLite/Postgres 切换。
- [ ] **Task 14.2: Redis Integration**
    - [ ] `common`: 引入 `redis` crate。
    - [ ] `router`: 将限流 (Rate Limiter) 和 令牌验证 (Token Validation) 迁移至 Redis (可选)。
- [ ] **Task 14.3: User Management & RBAC**
    - [ ] `server`: 实现用户注册、登录 (GitHub/OIDC)。
    - [ ] `database`: 设计用户角色表 (Role-Based Access Control)。

---
*Updated by AI Agent - LiveView Strategy*