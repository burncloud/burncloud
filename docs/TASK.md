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
- [x] **Task 9.1: Rate Limiter Middleware**
    - [x] `router`: 实现基于 Token 的限流 (TokenBucket/LeakyBucket).
    - [ ] `database`: Redis 集成准备 (可选，先基于内存).
- [x] **Task 9.2: Circuit Breaker (熔断器)**
    - [x] `router`: 自动检测上游连续失败并暂时剔除.
    - [x] `server`: 渠道健康状态监控 API (`/console/internal/health`).

## 📅 Phase 10: 本地模型管理完善 (Local Model Management)
- [x] **Task 10.1: Model Deletion UI**
    - [x] `client-models`: 绑定删除按钮事件，调用 `ModelService::delete` 清理数据库与文件。
- [x] **Task 10.2: File Download Integration**
    - [x] `client-models`: 在模型卡片中增加"文件列表"查看功能。
    - [x] `client-models`: 选择特定 GGUF 文件并触发下载 (调用 `service-models` 下载功能)。

## 📅 Phase 11: 本地推理服务 (Local Inference Service)
- [x] **Task 11.1: Inference Service Foundation**
    - [x] `service-inference`: 创建新的 Crate，负责管理本地推理进程 (llama-server).
    - [x] `service-inference`: 实现进程生命周期管理 (Start/Stop/Restart/Logs).
    - [x] `service-inference`: 自动检测可用的 llama-server 二进制文件 (或提供下载).
- [x] **Task 11.2: Local Upstream Registration**
    - [x] `service-inference`: 启动推理时，自动在 `router` 数据库中注册为 Upstream (localhost:port).
    - [x] `router`: 确保能路由到本地动态端口。
- [x] **Task 11.3: Deployment UI**
    - [x] `client-models`: 实现"Deploy"按钮逻辑，选择 GGUF 文件并启动服务。
    - [x] `client-models`: 展示正在运行的本地模型实例状态。

## 📅 Phase 12: 系统集成与测试 (System Integration & Testing)
- [x] **Task 12.1: End-to-End Testing**
    - [x] `tests`: 编写 E2E 测试脚本，覆盖"下载 -> 部署 -> 调用"全流程 (tests/e2e_flow.py).
    - [ ] `tests`: 使用 Python 或 Rust 编写外部调用脚本，验证 Router 的 OpenAI 兼容性。
- [x] **Task 12.2: CI/CD Configuration**
    - [x] `.github`: 完善 GitHub Actions，包含 Build, Test, Release 流程。
    - [ ] `.github`: 自动化构建 Windows 安装包 (msi/exe).
- [ ] **Task 12.3: Documentation**
    - [ ] `docs`: 更新用户手册 (User Guide)，说明如何添加模型、配置渠道。
    - [ ] `README.md`: 更新项目主页，添加最新功能介绍和截图。




---
*Updated by AI Agent - LiveView Strategy*
