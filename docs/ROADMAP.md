# BurnCloud 后端系统开发路线图 (Backend Roadmap)

**最后更新**: 2025-12-06
**参考架构**: New API (v0.10.0+) / LiteLLM
**当前版本**: v0.5.0-feature-complete

本路线图旨在构建一个**商业级、全功能、本地优先**的 LLM 后端系统。我们深度复刻 New API v0.10.0+ 的核心运营能力（分组、代理、返利），结合 Rust 的高性能特性，打造下一代 AI 网关。

> **图例**:
> - ✅ 已完成 (Completed)
> - 🚧 进行中 (In Progress)
> - 📅 计划中 (Planned)
> - 🔮 未来探索 (Future Exploration)

---

## 📅 第一阶段：基础设施固化 (Infrastructure Solidification)
*目标：统一数据访问层，引入 Redis，构建高并发基石。*

### 1.1 数据库层重构 (Database Layer)
- [x] **Router DB**: 重构 `burncloud-database-router` 为实例模式，修复 SQLx 类型兼容性。 ✅
- [ ] **User DB**: 重构 `burncloud-database-user`，移除静态方法，采用连接池。 📅
- [ ] **Redis Integration**: 引入 `redis` crate，实现连接池管理，用于热数据缓存（用户余额、Token 验证信息）。 📅
- [ ] **Migration System**: 建立基于 SQLx CLI 的迁移机制，支持 SQLite/Postgres 平滑升级。 📅

### 1.2 核心配置与环境 (Core Config)
- [ ] **Config Hot-Reload**: 完善热重载机制，支持不重启更新模型倍率和渠道配置。 📅
- [ ] **Environment Isolation**: 明确区分 `DEV` (SQLite) 和 `PROD` (Postgres) 模式。 📅

---

## 🚧 第二阶段：智能网关与会计系统 (Router Core & Accounting)
*目标：实现"预扣-累积-补偿"金融级计费，支持多级倍率与缓存加速。*

### 2.1 精准计费子系统 (Billing System v2)
- [ ] **Tiktoken Integration**: 集成 `tiktoken-rs`，实现本地 Token 计算。 📅
- [ ] **Multi-Level Ratio**: 实现 `FinalCost = (Prompt + Completion) * ModelRatio * GroupRatio`。 📅
- [ ] **Pre-deduct with Redis**: 优先从 Redis 原子扣除余额，异步落库。 📅
- [ ] **Stream Accumulator**: 在内存中拼接流式响应，计算真实 Token。 📅
- [ ] **Compensation Logic**: 请求结束时计算差额，执行"多退少补"。 📅

### 2.2 协议适配 (Protocol Adaptors)
- [x] **OpenAI Passthrough**: 标准透传。 ✅
- [x] **Gemini/Claude Adaptors**: 双向协议转换。 ✅
- [ ] **Vision Support**: 实现 GPT-4-Vision 的图片分辨率解析与计费。 🔮

### 2.3 流量治理 (Traffic Governance v2)
- [ ] **Group-based Routing (分组路由)**: 实现 `User.Group` 与 `Channel.Groups` 的匹配逻辑（如 VIP 用户只能用 VIP 渠道）。 📅
- [ ] **Per-Channel Proxy**: 支持为每个渠道单独配置 HTTP/SOCKS5 代理，解决网络连通性问题。 📅
- [ ] **Priority & Weight**: 支持优先级故障转移与权重负载均衡。 📅
- [ ] **Content Filtering**: 简单的关键词过滤中间件，拦截违规 Prompt 或 Completion。 📅

---

## 📅 第三阶段：业务服务体系 (Service Ecosystem)
*目标：提供完整的运营管理能力。*

### 3.1 用户与鉴权 (Identity & Access)
- [ ] **Unified Auth**: GitHub OAuth + Email/Password。 📅
- [ ] **SMTP Service**: 集成邮件发送能力，支持注册验证码、密码重置与余额预警。 📅
- [ ] **Affiliate System (邀请返利)**: 实现邀请链接注册，记录邀请关系与返佣逻辑。 📅
- [ ] **Redemption Code**: 兑换码系统，支持批量生成与导出。 📅

### 3.2 渠道管理 (Channel Management)
- [ ] **Smart Health Check**: 
    - 定时探测渠道连通性与响应时间 (Latency)。
    - 连续失败 N 次后自动禁用，恢复后自动启用。 📅
- [ ] **Model Mapping**: 支持模型重定向（如 `gpt-3.5` -> `gpt-3.5-turbo`）与自定义映射。 📅
- [ ] **Tagging System**: 为渠道添加标签，便于前端筛选展示。 📅

### 3.3 系统可观测性与维护 (Ops)
- [ ] **Data Export/Import**: 支持导出所有配置（渠道、用户、倍率）为 JSON，便于迁移。 📅
- [ ] **Async Logging**: 异步写入日志到数据库，不阻塞主线程。 📅
- [ ] **Operational Dashboard**: 展示 RPS、TPM、错误率图表。 📅

---

## 🔮 第四阶段：异步任务与扩展 (Async Tasks & Extensions)
*目标：支持 Midjourney 等异步模型。*

- [ ] **Task Queue**: 引入任务队列机制，处理 Midjourney 的 Imagine/Upscale/Variation 异步任务。 🔮
- [ ] **Plugin System**: 支持通过 Wasm 加载自定义的过滤或处理插件。 🔮

---

*本路线图深度对标 New API v0.10.0+ 架构，补充了分组路由、独立代理、邀请返利等关键运营特性。*