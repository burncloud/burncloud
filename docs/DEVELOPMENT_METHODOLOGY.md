# BurnCloud 开发方法论：从 New API 到 Rust 的演进之路

**日期**: 2025-12-07
**状态**: 核心指导文档 (Core Guideline)
**目标**: 指导如何以 **"行为级复刻，架构级重写"** 的策略，将 New API (Go) 的功能完美迁移至 BurnCloud (Rust)，同时利用 Rust 优势进行架构升级。

---

## 1. 核心原则 (Core Principles)

**❌ 绝对禁止**:
- **逐行直译**: 不要在 Rust 里写 "Go 风格的代码" (Go-ism)。拒绝滥用 `Any` (interface{})，拒绝全局变量，拒绝在逻辑层混杂数据库操作。
- **盲目照搬 Bug**: New API 某些为了快速迭代而产生的"面条代码" (Spaghetti Code) 必须被重构。

**✅ 核心策略**:
- **接口 100% 兼容 (API Compatibility)**: 对外暴露的 HTTP 接口（路径、参数、Header、错误码）必须与 New API 严格一致，确保客户端零感知。
- **数据结构优先 (Data-First)**: 先复刻数据模型 (Structs) 和数据库 Schema，再填充业务逻辑。
- **Rust 范式 (Idiomatic Rust)**: 使用 `Trait` 替代 Interface，使用 `Enum` 替代 String 类型判断，使用 `Result/Option` 处理错误与空值，使用 `SQLx` 获得编译期类型安全。

---

## 2. 执行路线图 (Execution Plan)

### 阶段一：数据与标准定义 (Data & Standards)
*目标：建立两者的"通用语言"——数据结构。*

1.  **Struct 迁移**: 
    - 将 Go 的 `model/*.go` 转换为 Rust 的 `struct` (使用 `serde` 进行序列化控制)。
    - **拆分大对象**: New API 的 `User` 对象过于臃肿，Rust 中应根据领域上下文拆分为 `UserAuth`, `UserQuota`, `UserSettings`。
2.  **Enum 标准化**:
    - 将 Go 中的 `const` (如 `ChannelTypeOpenAI = 1`) 转换为 Rust 的强类型 `enum ChannelType { OpenAI = 1, ... }`，利用 `strum` crate 实现自动转换。

### 阶段二：核心引擎重构 (Core Engine Reconstruction)
*目标：重写系统的"心脏"——路由与转发。*

1.  **路由系统 (Router)**:
    - 复刻 `abilities` 表设计。
    - 实现基于 `Group + Model` 的高频路由查找算法（Rust `HashMap` 读性能极高）。
2.  **适配器模式 (Adaptor Trait)**:
    - 定义标准 Trait：
      ```rust
      #[async_trait]
      pub trait ModelAdaptor: Send + Sync {
          // 生成上游请求
          async fn generate_request(&self, ctx: &RelayContext) -> Result<UpstreamRequest>;
          // 处理上游响应（流式/非流式）
          async fn handle_response(&self, resp: UpstreamResponse) -> Result<AxumResponse>;
      }
      ```
    - 利用 Rust 的泛型和 Enum Dispatch 消除运行时开销。

### 阶段三：基础设施升级 (Infrastructure Upgrade)
*目标：解决 New API 的痛点。*

1.  **数据库层**: 
    - 放弃 ORM (GORM)，拥抱 **SQLx**。编写原始 SQL 以获得极致性能和确定的执行计划。
    - 支持 **Redb** (嵌入式) 与 **Redis** (分布式) 的双模缓存架构。
2.  **并发模型**:
    - 利用 Tokio 运行时，将所有 IO 操作（数据库、HTTP 请求）完全异步化。
    - 避免全局锁 (Global Mutex)，使用 `Actor` 模式或细粒度 `RwLock` 管理状态。

---

## 3. 迁移对照表 (Migration Cheatsheet)

| New API (Go) | BurnCloud (Rust) | 备注 |
| :--- | :--- | :--- |
| `gin` (Web Framework) | `axum` | Axum 类型安全更强，基于 Tower 生态。 |
| `gorm` (ORM) | `sqlx` | SQLx 编译期检查 SQL，无运行时反射开销。 |
| `interface{}` | `Enum` / `Trait` | Rust 必须明确类型，处理 `Any` 非常痛苦。 |
| `goroutine` | `tokio::spawn` | 概念类似，但 Rust Future 是惰性的。 |
| `defer recover()` | `Result<T, E>` | Rust 没有 Panic 恢复机制用于控制流，必须显式处理错误。 |
| `channel.go` (Consts) | `enums/channel.rs` | 使用 `num_enum` 或 `strum` 宏。 |

---

## 4. 开发工作流 (Workflow)

1.  **分析 (Analyze)**: 阅读 New API 对应功能的 Go 源码，理解其**输入**、**输出**和**副作用** (修改了哪个表？扣了多少费？)。
2.  **定义 (Define)**: 在 Rust 中定义对应的 Request/Response 结构体和 Trait 接口。
3.  **实现 (Implement)**: 编写 Rust 逻辑，尽量使用函数式编程 (`map`, `filter`, `fold`) 简化逻辑。
4.  **测试 (Verify)**: 编写集成测试，模拟 HTTP 请求，对比 New API 的响应结果。

---

## 5. 测试策略 (Testing Strategy)

### 5.1 真实环境优先 (Real-World First)
- **禁止使用 Mock Server**: 在 E2E 测试中，不再使用 `wiremock` 等进程内 Mock 工具，避免因网络环境差异导致的 False Negative。
- **基于配置的测试**: 测试用例应从 `.env` 读取 `TEST_UPSTREAM_BASE_URL` 和 `TEST_UPSTREAM_KEY`。
- **按需跳过**: 如果 `.env` 中未配置真实的上游信息，测试应打印警告并 `return Ok(())` (Skip)，而不是报错。

### 5.2 测试配置 (.env)
开发者应在 `.env` 中提供以下字段以启用完整测试：
```env
TEST_OPENAI_BASE_URL=https://api.openai.com
TEST_OPENAI_KEY=sk-your-real-key
```

---

*此文档作为 BurnCloud 后续开发的核心准则。*