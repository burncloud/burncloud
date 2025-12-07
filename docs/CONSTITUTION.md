# BurnCloud 项目开发宪法 (Development Constitution)

**版本**: 1.9
**生效日期**: 2025-12-07

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

### 3.4 协议适配的可选性 (Protocol Adaptor Optionality)
*   **默认透传**: 如果客户端使用的是原生协议（如 Gemini SDK 访问 Gemini），Router **绝不**进行任何格式转换。
*   **显式触发**: 仅当用户明确需要（例如通过配置或请求头指定“模拟 OpenAI”）时，才启用协议适配器 (Adaptor) 进行 Request/Response 转换。

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
*   `✨ feat: add aws sigv4 signing support`
*   `📚 docs: update CONSTITUTION.md`

### 4.2 测试规范 (Testing Standards)
*   **单元测试 (Mandatory Unit Tests)**：每次完成原子级开发任务时，必须编写单元测试来验证核心逻辑。
*   **集成测试**：必须与生产代码分离。
    *   **数据源隔离**：测试所需的 API Key 或凭据必须通过**环境变量** (如 `TEST_AWS_AK`) 注入，或者从测试专用的临时数据库中读取。
    *   **严禁硬编码**：源代码中（包括 `tests/` 目录下的文件）**绝对不应该出现**任何真实的 API Key、Access Key/Secret Key。
    *   **强制环境变量**：所有测试用例中的敏感凭据必须通过 `std::env::var` 获取，严禁为了方便而临时硬编码。
    *   **敏感信息还原**：如果在本地调试或紧急修复过程中临时写入了真实 Key，**必须在提交代码或任务结束前，将其还原为脱敏的样例 Key (如 `YOUR_AK_HERE`)**。
    *   **严禁敏感文件**：严禁在代码库中包含存储了真实密钥的 JSON/YAML/ENV 文件。
    *   测试必须具备幂等性，且不能污染用户的真实数据库。
*   **E2E 测试位置与结构 (E2E Testing Location & Structure)**：
    *   **强制位置**：所有的 E2E (End-to-End) 测试文件必须存放在项目根目录的 `/crates/tests` 文件夹中。
    *   **路径对应**：E2E 测试文件的命名和目录结构必须严格对应其测试的 Router 请求路径。
        *   例如：测试路由 `POST /v1/chat/completions` 的 E2E 测试，应位于 `/crates/tests/v1/chat/completions_test.rs`。
        *   例如：测试路由 `GET /api/models` 的 E2E 测试，应位于 `/crates/tests/api/models_test.rs`。
*   **提交前测试 (Test Before Commit)**：在标记任务完成前，必须确保 `cargo test` 通过。

### 4.3 错误处理
*   使用 `anyhow` 处理顶层错误，使用 `thiserror` 定义库级别错误。
*   严禁使用 `unwrap()`，除非在 Test 代码中或你有 100% 的把握它不会 Panic（并写上注释说明原因）。

### 4.4 原子化开发 (Atomic Development)
*   **最小单元**: 每次开发必须以“最小可行单元”为粒度（例如：只支持 DeepSeek 的 AuthType，而不是一次性支持所有国产模型）。
*   **开发闭环**: 必须遵循 `Plan` -> `Code` -> `Test` -> `Commit` 的完整闭环。上一个单元未通过测试前，严禁开始下一个单元的开发。
*   **分步提交**: 避免“大爆炸”式的代码提交。

### 4.5 零警告容忍 (Zero Warning Tolerance)
*   **Rustc/Clippy Clean**: 代码必须能够通过 `cargo check` 和 `cargo clippy` 而不产生任何 Warning。
*   **清理无用代码**: 严禁保留未使用的引用 (`unused imports`)、变量或死代码。
*   **命名规范**: 严格遵守 Rust 的命名惯例 (如 snake_case)。

### 4.6 代码排版 (Code Layout)
*   **严禁无谓换行**: 铁律——不允许代码中出现无谓的换行。代码应保持紧凑，仅在逻辑块之间保留必要的空行。

### 4.7 依赖管理 (Dependency Management)
*   **版本统一**: 所有依赖包的版本号必须在根目录 `Cargo.toml` 的 `[workspace.dependencies]` 中统一声明。
*   **禁止散落**: 子 Crate 的 `Cargo.toml` 必须使用 `workspace = true` 引用依赖，严禁在子 Crate 中硬编码版本号。

### 4.8 营销优先 (Marketing First)
*   **文案优先**: 每次更新 `README.md` 时，必须优先考虑营销文案，清晰突出核心功能 (Features) 和用户价值 (Benefits)。
*   **视觉吸引**: 善用徽章 (Badges)、Emoji 和清晰的排版，确保文档对开发者和用户具有吸引力。

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