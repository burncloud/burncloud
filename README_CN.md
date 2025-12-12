# BurnCloud (奔云)

<div align="center">

![Rust](https://img.shields.io/badge/Built_with-Rust-orange?style=for-the-badge&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)
![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blue?style=for-the-badge)
![Tests](https://img.shields.io/badge/Tests-Passing-success?style=for-the-badge)

**The Next-Gen High-Performance AI Gateway & Aggregator**
**下一代高性能大模型聚合网关与管理平台**

[Feature Requests](https://github.com/burncloud/burncloud/issues) · [Roadmap](docs/ARCHITECTURE_EVOLUTION.md) · [Documentation](docs/)

[English](README.md) | [简体中文](README_CN.md)

</div>

---

## 💡 What is BurnCloud?

BurnCloud 是一个 **Rust 原生** 的大模型聚合网关与管理平台。
它的目标是对标并超越 **One API (New API)**，为个人开发者、团队和企业提供一个**高性能、低资源占用、安全可控**的 LLM 统一接入层。

**我们不仅仅是造轮子，我们是在升级引擎。**
如果你受够了现有网关的高内存占用、GC 停顿或复杂的部署依赖，BurnCloud 是你的最佳选择。

## ✨ Why BurnCloud? (核心价值)

### 🚀 1. 极致性能 (Performance First)
*   **Rust 驱动**: 基于 `Axum` 和 `Tokio` 构建，拥有惊人的并发处理能力和极低的内存占用（MB 级别 vs GB 级别）。
*   **零损耗透传**: 独创的 "Don't Touch the Body" 路由模式，在非协议转换场景下，实现字节级零拷贝转发，延迟近乎为零。
*   **单二进制文件**: 没有任何 Runtime 依赖（无 Python、无 Node.js、无 Java），一个文件即是一个完整的平台。

### 🔌 2. 万能聚合 (Universal Aggregation)
*   **All to OpenAI**: 将 Anthropic (Claude), Google (Gemini), Azure, 阿里 Qwen 等所有主流模型的协议统一转换为标准 **OpenAI 格式**。
*   **一次接入，处处运行**: 你的 LangChain、AutoGPT 或任何现有应用，只需修改 Base URL 即可无缝切换底层模型。

### ⚖️ 3. 运营级治理 (Enterprise Control)
*   **智能负载均衡**: 支持多渠道轮询 (Round-Robin)、权重分发 (Weighted) 和 自动故障转移 (Failover)。一个 `gpt-4` 倒下了，千千万万个 `gpt-4` 站起来。
*   **精准计费**: 支持基于 Token 的精准扣费、自定义倍率 (Model Ratio) 和用户分组倍率 (Group Ratio)。
*   **多租户管理**: 完善的兑换码、额度管理、邀请机制。

### 🛡️ 4. 坚若磐石 (Rock-Solid Reliability)
*   **真实 E2E 测试**: 我们抛弃了虚假的 Mock 数据。BurnCloud 的 CI/CD 流程直接对接真实的 OpenAI/Gemini API 进行端到端验证，确保核心转发逻辑在真实网络环境下依然健壮。
*   **浏览器驱动验证**: 内置基于 **Headless Chrome** 的自动化 UI 测试，确保从后端 API 到前端 Dioxus LiveView 的渲染链路畅通无阻。
*   **零回归承诺**: 严格的 **"API-Path Matching"** 测试策略，每一次 Commit 都经过了严苛的自动化审计。

### 🎨 5. 优雅体验 (Fluent Experience)
*   **不仅仅是 API**: 内置基于 **Dioxus** 开发的 **Windows 11 Fluent Design** 本地管理客户端。
*   **可视化监控**: 实时查看 TPS、RPM、令牌消耗趋势，告别枯燥的日志文件。

---

## 🏗️ Architecture (架构)

BurnCloud 采用严格的四层架构设计，确保高内聚低耦合：

*   **Gateway Layer (`crates/router`)**: 数据面。处理高并发流量，负责鉴权、限流、协议转换。
*   **Control Layer (`crates/server`)**: 控制面。提供 RESTful API 供 UI 调用，管理配置与状态。
*   **Service Layer (`crates/service`)**: 业务面。封装计费、监控、渠道测速等核心逻辑。
*   **Data Layer (`crates/database`)**: 数据面。基于 SQLx + SQLite/PostgreSQL，未来支持 Redis 缓存。

> 详见: [架构演进文档 (Architecture Evolution)](docs/ARCHITECTURE_EVOLUTION.md)

---

## 🛠️ Getting Started (快速开始)

### 环境要求
*   Rust 1.75+
*   Windows 10/11, Linux, or macOS

### 开发模式运行

```bash
# 1. 克隆项目
git clone https://github.com/burncloud/burncloud.git
cd burncloud

# 2. 配置 (可选)
cp .env.example .env
# 编辑 .env 填入 TEST_OPENAI_KEY 以启用完整 E2E 测试

# 3. 运行 (自动编译 Server 和 Client)
cargo run
```

### 运行测试 (Quality Assurance)

体验工业级测试流程：

```bash
# 运行所有 API 集成测试
cargo test -p burncloud-tests --test api_tests

# 运行 UI 自动化测试 (需 Chrome)
cargo test -p burncloud-tests --test ui_tests
```

---

## 🗺️ Roadmap (路线图)

- [x] **v0.1**: 基础路由与 AWS SigV4 签名支持 (已完成)
- [x] **v0.2**: 数据库集成、基础鉴权与 **New API 核心复刻** (已完成)
    - [x] Ability 智能路由
    - [x] Channel 管理 API
    - [x] 异步计费日志
- [x] **v0.3**: 统一协议适配器 (OpenAI/Gemini/Claude) & E2E 测试体系 (已完成)
- [ ] **v0.4**: 智能负载均衡与故障转移 (进行中)
- [ ] **v0.5**: Web 控制台前端完善
- [ ] **v1.0**: 正式发布，Redis 缓存集成

---

## 🤝 Contributing

我们欢迎任何形式的贡献！请务必在提交代码前阅读我们的 **[开发宪法 (Constitution)](docs/CONSTITUTION.md)**。

## 📄 License

MIT License © 2025 BurnCloud Team
