# Tasks: BurnCloud Router Implementation

## Phase 1: Initialization & Scaffold
- [x] **创建 Crate**
    - 在 `crates/` 下初始化 `router` 项目: `cargo new crates/router --lib`.
    - 更新根目录 `Cargo.toml` 的 workspace members。
    - 添加基础依赖: `axum`, `tokio`, `reqwest`, `tracing`, `serde`, `thiserror`.
- [x] **基础 HTTP 服务**
    - 实现 `start_server` 函数。
    - 编写一个简单的 "Hello World" Axum handler 确保服务可运行。

## Phase 2: Core Proxy Logic (The "Passthrough")
- [x] **实现通用 Proxy Handler**
    - 创建一个接收 `Any` 路径和方法的 Handler: `async fn proxy_handler(uri: Uri, method: Method, ...)`
    - 使用 `reqwest` 构建向下游的请求。
    - **关键**: 确保 `Body` 是流式透传的 (Streaming)，不要一次性读取到内存。
- [x] **Header 处理**
    - 实现 Header 过滤逻辑 (移除 Host, Connection 等 Hop-by-hop headers)。
    - 实现 CORS 处理 (允许跨域)。

## Phase 3: Routing & Configuration
- [x] **定义配置结构体**
    - `struct Upstream`: 包含 `base_url`, `api_key`, `match_path`.
    - `struct RouterConfig`:包含 Upstream 列表。
- [x] **实现路由匹配引擎**
    - 编写逻辑：根据传入的 `Request Path`，在配置列表中查找匹配的 Upstream。
    - 单元测试：测试不同路径 (`/v1/chat/completions`, `/v1/messages`) 是否能正确匹配到对应的 Upstream。

## Phase 4: Authentication & Key Injection
- [x] **设计 Token 验证中间件**
    - 从 Request Header 中提取 `Authorization: Bearer sk-burncloud...`。
    - 验证 Token 是否有效 (暂时可使用硬编码或内存 Map 测试)。
- [x] **密钥注入逻辑**
    - 根据 Upstream 的类型 (`OpenAI`, `Claude`, `Gemini`)，构造正确的鉴权 Header。
    - *OpenAI*: `Authorization: Bearer sk-xxx`
    - *Claude*: `x-api-key: sk-xxx`, `anthropic-version: 2023-06-01`
    - *Google*: Query parameter `?key=xxx` 或 Header。

## Phase 5: Database Integration
- [x] **设计数据库表**
    - 在 `crates/database` 中添加新的 Migration 或 Table 定义 (`router_upstreams`, `router_tokens`).
    - 更新 `burncloud-database` crate 导出相关操作接口。
- [x] **连接数据库**
    - 将 `sqlx::Pool` 注入到 Router 的 State 中。
    - 替换内存中的 Token 验证逻辑为数据库查询。

## Phase 6: Logging & Accounting (Deferred)
- [ ] **访问日志**
    - 使用 `tracing` 记录每次请求的源 IP、目标 Upstream、耗时和状态码。
- [ ] **简单计费**
    - 创建异步任务，在请求结束后更新数据库中的使用量 (Usage Count / Bytes)。

## Phase 7: Integration & Testing
- [x] **集成到主程序**
    - 在 `src/main.rs` 中添加新的 CLI 命令或启动逻辑 (e.g., `burncloud router`).
- [ ] **端到端测试**
    - 使用 `curl` 或 Postman 模拟客户端。
    - 验证流式响应是否流畅。
    - 验证不同厂商 API 是否能正确路由。

## Phase 8: AWS Bedrock Support (New)
- [ ] **引入 AWS 依赖**
    - 添加 `aws-sigv4`, `aws-credential-types`, `aws-smithy-http` (或相关 HTTP 类型的转换库) 到 `crates/router/Cargo.toml`。
- [ ] **更新鉴权类型**
    - 在 `AuthType` 枚举中添加 `AwsSigV4`。
    - 实现凭证解析逻辑：将 `api_key` 字符串解析为 `AccessKey:SecretKey:Region`。
- [ ] **实现签名逻辑 (Signer)**
    - 创建 `aws_signer` 模块。
    - 实现请求正文的缓冲 (Buffering)，因为 AWS 签名需要 Body 的 SHA256 哈希。
    - 使用 `aws-sigv4` 对 HTTP Request 进行签名，生成 `Authorization` 和 `X-Amz-Date` 头。
- [ ] **集成 AWS 路由**
    - 在 `proxy_handler` 中添加分支处理 `AwsSigV4`。
    - 确保上游 URL 是 Bedrock 的标准格式 (e.g., `bedrock-runtime.{region}.amazonaws.com`)。
