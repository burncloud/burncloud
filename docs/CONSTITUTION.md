# BurnCloud 项目开发宪法 (Development Constitution)

**版本**: 1.2
**生效日期**: 2025-12-05

## 序言

BurnCloud (奔云) 不仅仅是一个工具，它是一个致力于让大模型本地化部署和管理变得优雅、高效且可控的平台。本宪法确立了项目的核心价值观、架构原则和开发规范，所有贡献者（包括人类开发者和 AI 助手）都必须遵守。

---

## 第一章：核心哲学 (Core Philosophy)

### 1.1 性能至上 (Performance First)
我们选择 Rust 是因为对性能的极致追求。
*   **拒绝臃肿**：在引入依赖时必须极其克制。能用标准库解决的，不引入第三方库。
*   **异步优先**：所有 I/O 操作（网络、文件、数据库）必须是异步的 (Tokio-based)。

### 1.2 本地优先 (Local First)
*   用户的数据属于用户。所有的配置、数据库 (SQLite)、模型文件都存储在用户本地。
*   不上传任何遥测数据或用户隐私数据，除非用户明确授权。

### 1.3 优雅的交互 (Fluent Experience)
*   UI 必须遵循 Windows 11 Fluent Design 设计语言。
*   响应迅速，视觉反馈流畅，拒绝卡顿。

---

## 第二章：架构原则 (Architectural Principles)

项目采用 **Rust Workspace (Monorepo)** 结构，遵循严格的分层架构。

### 2.1 模块化 (Modularization)
严禁构建单体巨石应用。功能必须拆分为独立的 Crate：
*   **UI 层 (`crates/client`)**: 只负责渲染和交互，不包含核心业务逻辑。
*   **服务层 (`crates/service`)**: 纯 Rust 业务逻辑，无 UI 依赖。
*   **数据层 (`crates/database`)**: 负责持久化，使用 SQLx。
*   **路由层 (`crates/router`)**: 独立的高性能网关组件。
*   **核心层 (`crates/core`)**: 共享的底层逻辑。

### 2.2 依赖单向流动
依赖关系必须清晰且单向：
`Client` -> `Service` -> `Database/Core`
禁止反向依赖或循环依赖。

---

## 第三章：Router 组件特别法 (The Router Doctrine)

针对 `crates/router` 组件，确立以下不可动摇的原则：

### 3.1 透传原则 (Passthrough Principle)
**"Don't Touch the Body" (不触碰包体)** 是 Router 的最高准则。
*   我们是一个**智能管道**，不是处理器。
*   **严禁**对 Request/Response Body 进行 JSON 解析、反序列化或重组（除非鉴权机制强制要求，如 AWS SigV4）。
*   保持流式 (Streaming) 响应的绝对畅通，确保打字机效果零延迟。

### 3.2 极简协议适配
*   不试图统一各家厂商的 JSON 格式。
*   Router 负责**路由分发**、**鉴权替换**和**计费统计**。
*   客户端决定它在使用什么格式（OpenAI SDK 用 OpenAI 格式，Claude SDK 用 Claude 格式）。

### 3.3 独立性与轻量化
*   AWS 等复杂鉴权逻辑必须隔离在子 Crate 中（如 `router-aws`）。
*   避免引入庞大的 SDK（如完整的 AWS SDK），优先使用轻量级的 crypto 库手写实现签名逻辑，以保持二进制文件的体积和编译速度。

---

## 第四章：工程与代码规范 (Engineering Standards)

### 4.1 提交规范 (Commit Standards)
Git 提交信息必须遵循 **Emoji Prefix** 格式，并在描述中清晰说明变更内容。

**格式**: `<Icon> <Type>: <Summary>`

| 图标 | 类型 (Type) | 说明 |
| :--- | :--- | :--- |
| ✨ | `feat` | 新功能 (New Feature) |
| 🐛 | `fix` | Bug 修复 |
| 📚 | `docs` | 文档变更 |
| 🔨 | `refactor` | 代码重构 (无功能变更) |
| 🚀 | `perf` | 性能优化 |
| 🧪 | `test` | 测试代码变更 |
| 🔧 | `chore` | 构建过程或辅助工具变更 |

**示例**:
*   ✅ `✨ feat: add aws sigv4 signing support`
*   ✅ `📚 docs: update CONSTITUTION.md`

### 4.2 测试规范 (Testing Standards)
*   **单元测试**：核心逻辑（如 URL 编码、签名计算、配置解析）必须有单元测试覆盖。
*   **集成测试**：必须与生产代码分离。
    *   **数据源隔离**：测试所需的 API Key 或凭据必须通过**环境变量** (如 `TEST_AWS_AK`) 注入，或者从测试专用的临时数据库中读取。
    *   **严禁硬编码**：源代码中（包括 `tests/` 目录下的文件）**绝对不应该出现**任何真实的 API Key、Access Key/Secret Key。
    *   **严禁敏感文件**：严禁在代码库中包含存储了真实密钥的 JSON/YAML/ENV 文件。
    *   测试必须具备幂等性，且不能污染用户的真实数据库。

### 4.3 错误处理
*   使用 `anyhow` 处理顶层错误，使用 `thiserror` 定义库级别错误。
*   严禁使用 `unwrap()`，除非在 Test 代码中或你有 100% 的把握它不会 Panic（并写上注释说明原因）。

---

## 第五章：安全准则 (Security Protocols)

### 5.1 凭据管理
*   **零信任**：所有的 API Key 在数据库中建议加密存储（未来规划）。
*   **零泄露**：Git 历史中严禁出现真实的 Access Key 或 Secret Key。如果不慎提交，必须立即废弃该密钥并重写 Git 历史。

### 5.2 鉴权
*   Router 必须验证用户的 Bearer Token 才能转发请求。
*   对外暴露的端口默认绑定 `127.0.0.1`，除非用户显式配置为 `0.0.0.0`。

---

## 第六章：AI 代理行为准则 (AI Agent Protocol)

所有协助开发的 AI 代理在完成任务时必须遵守以下汇报流程：

### 6.1 汇报语言 (Reporting Language)
*   无论用户使用何种语言提问，AI 代理在**总结更新内容**时必须使用 **中文 (Chinese)**。
*   这有助于保持项目文档和沟通的一致性（项目核心语言为中文）。

### 6.2 提交信息生成 (Git Message Generation)
*   在每次回复的末尾，AI 代理必须提供一段**英文**的 Git Commit Message。
*   该 Message 必须严格遵守 **4.1 提交规范** 中的 Emoji 格式。
*   这方便开发者直接复制粘贴进行提交。

---

## 附录：目录结构映射

```
burncloud/
├── crates/
│   ├── client/          # Dioxus GUI (Fluent Design)
│   ├── router/          # LLM Passthrough Gateway (Axum)
│   │   └── crates/router-aws # AWS SigV4 Logic
│   ├── service/         # Business Logic
│   ├── database/        # SQLx/SQLite Logic
│   └── common/          # Shared Types
├── tests/               # End-to-End Integration Tests
└── src/                 # Application Entry (main.rs)
```

---
*本宪法由 BurnCloud 架构师确立，所有后续开发均需以此为基石。*
