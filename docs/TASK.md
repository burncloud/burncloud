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

## 📅 Phase 6: Web UI 架构重构 (LiveView Transition)
**目标**: 放弃 Desktop/WASM 路线，全面转向 **Dioxus LiveView**。将 UI 渲染逻辑移至服务端，通过 Axum + WebSocket 提供无需安装的纯 Web 管理界面，实现“开箱即用”的 OneAPI 体验。

- [ ] **Task 6.1: Dependency Overhaul**
    - [ ] `crates/client`: 移除 `dioxus-desktop`，引入 `dioxus-liveview` 和 `axum`。
    - [ ] `crates/client`: 重构 `Cargo.toml`，清理不再需要的桌面端依赖（如 `tray`）。

- [ ] **Task 6.2: LiveView Server Integration**
    - [ ] `crates/client/src/lib.rs`: 导出一个 `launch_liveview_router(pool: Pool<Sqlite>) -> Router` 函数。
    - [ ] `crates/client`: 修改 `app.rs` 以适应 LiveView 渲染模式（移除 Window 相关代码）。
    - [ ] `crates/server/src/lib.rs`: 引入 `burncloud-client`，并将 LiveView 路由挂载到根路径 `/`。

- [ ] **Task 6.3: Direct Database Integration**
    - [ ] `crates/client`: 逐步移除 `ApiClient` (HTTP)，改为在组件 Server 端直接调用 `RouterDatabase`。
    - [ ] *好处*: 不需要序列化 JSON，不需要 HTTP 往返，性能更高，代码更少。

- [ ] **Task 6.4: UI Cleanup & Enhancement**
    - [ ] 修复因移除 Desktop 而失效的组件（如系统托盘）。
    - [ ] 确保 `styles.css` 在 LiveView 模式下正确加载（通过 HTML Head 注入）。

---

## 📅 Phase 5: 精确计费与日志 (Billing & Logging)
*(保持不变)*

---
*Updated by AI Agent - LiveView Strategy*
