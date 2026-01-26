[
    {
        "category": "ui",
        "description": "实现模型部署(Deploy)页面的E2E测试（需登录态，全程Mock API）",
        "steps": [
            "创建 deploy.spec.ts，在 beforeEach 中设置已登录的 Auth Token，并访问 /deploy",
            "拦截 POST 请求 '/api/v1/models/deploy'，Mock 返回 { success: true, modelId: 'gpt2' }",
            "验证页面标题包含 'Deploy'，且初始状态下 getByRole('button', { name: /Deploy/i }) 为 disabled",
            "使用 getByPlaceholder 或 getByLabel 填写 Model ID 为 'gpt2'，并选择 Source 为 'HuggingFace'",
            "断言 'Deploy' 按钮变为 enabled，点击该按钮",
            "等待 Mock 接口调用完成，验证页面出现包含 'Deployment Successful' 文本的 Toast 提示",
            "验证页面 URL 最终跳转至 '/models'"
        ],
        "passes": true
    },
    {
        "category": "ui",
        "description": "实现模型列表(Models)及管理的E2E测试（需登录态，全程Mock API）",
        "steps": [
            "创建 models.spec.ts，在 beforeEach 中访问 /models",
            "拦截 GET 请求 '/api/v1/models'，Mock 返回包含一个名为 'gpt2', 状态为 'Running' 的数据列表",
            "拦截 DELETE 请求 '/api/v1/models/gpt2'，Mock 返回 { success: true }",
            "验证页面渲染出的表格包含 'Status', 'Name', 'Replicas', 'Actions' 等列头",
            "断言 'gpt2' 这一行的状态文本为 'Running'",
            "点击 'gpt2' 这一行中 name='Delete' 的操作按钮",
            "断言出现删除确认的 Modal 弹窗，点击 Modal 中的 'Confirm' 按钮",
            "等待 DELETE 请求完成，断言 'gpt2' 这一行从 DOM 树中消失"
        ],
        "passes": true
    },
    {
        "category": "ui",
        "description": "实现Playground页面的E2E测试（需登录态，全程Mock API）",
        "steps": [
            "创建 playground.spec.ts，在 beforeEach 中访问 /playground",
            "拦截 POST 请求 '/api/v1/chat/completions'，Mock 返回 { choices: [{ message: { content: 'Mock AI Response' } }] }",
            "使用 getByPlaceholder 定位聊天输入框，输入 'Hello World'",
            "断言发送按钮(Role: button, Name: Send)可用，并点击发送",
            "断言输入框内容被清空，且页面中出现包含 'Hello World' 的用户消息气泡",
            "等待 POST 请求完成，断言页面中出现包含 'Mock AI Response' 的 AI 回复气泡"
        ],
        "passes": true
    },
    {
        "category": "ui",
        "description": "实现日志(Logs)页面的E2E测试（需登录态，全程Mock API）",
        "steps": [
            "创建 logs.spec.ts，在 beforeEach 中访问 /logs",
            "拦截 GET 请求 '/api/v1/logs'，Mock 返回包含 'INFO: App started' 和 'ERROR: Connection failed' 的日志列表",
            "拦截 GET 请求 '/api/v1/logs?q=ERROR'，Mock 返回仅包含 'ERROR: Connection failed' 的数据",
            "在页面加载后，断言页面中显示了 2 条日志记录",
            "使用 getByPlaceholder 定位搜索框，输入 'ERROR' 并触发搜索/回车",
            "等待查询请求完成，断言日志列表被过滤为 1 条，且文本包含 'ERROR'",
            "点击清除搜索按钮，断言列表恢复为 2 条日志"
        ],
        "passes": true
    },
    {
        "category": "ui",
        "description": "实现账单(Billing)页面的E2E测试（需登录态，全程Mock API）",
        "steps": [
            "创建 billing.spec.ts，在 beforeEach 中访问 /settings/billing",
            "拦截 GET 请求 '/api/v1/billing/balance'，Mock 返回 { balance: 50000, currency: 'USD' }",
            "拦截 POST 请求 '/api/v1/billing/recharge'，Mock 返回 { success: true, new_balance: 51000 }",
            "页面加载后，断言余额组件中显示文本 '$50,000'",
            "点击 name='Recharge' 的按钮，断言弹窗打开，在金额输入框输入 1000，点击提交",
            "等待 POST 请求完成，断言余额文本变更为 '$51,000' 或出现 '充值成功' 的提示"
        ],
        "passes": true
    },
    {
        "category": "ui",
        "description": "实现开发者设置(API Keys)的E2E测试（需登录态，全程Mock API）",
        "steps": [
            "创建 api_keys.spec.ts，在 beforeEach 中访问 /settings/api-keys",
            "拦截 GET 请求 '/api/v1/keys'，Mock 返回空列表 []",
            "拦截 POST 请求 '/api/v1/keys'，Mock 返回 { id: 'key_1', name: 'Test Key', secret: 'sk-1234567890abcdef' }",
            "点击 getByRole('button', { name: 'Generate New Key' }) 按钮",
            "在弹出的表单中输入 Key 名称 'Test Key' 并提交",
            "等待 POST 请求完成后，断言列表中新增一行 'Test Key'",
            "断言该行的 Secret 部分被掩码处理，包含文本 'sk-***' 或 'sk-123...'"
        ],
        "passes": true
    },
    {
        "category": "backend",
        "description": "Task 1.1: 添加 Google Vertex AI 所需依赖",
        "steps": [
            "修改 `crates/router/Cargo.toml`，添加 `jsonwebtoken = \"9\"`",
            "更新 workspace 依赖配置 (如果需要)",
            "验证: `cargo build -p burncloud-router` 成功下载并编译依赖"
        ],
        "passes": true
    },
    {
        "category": "backend",
        "description": "Task 1.2: 创建 VertexAdaptor 结构体并实现基础 trait",
        "steps": [
            "创建 `crates/router/src/adaptor/vertex.rs`",
            "定义 `pub struct VertexAdaptor;`",
            "实现 `ChannelAdaptor` trait，name() 返回 'VertexAi'",
            "验证: 文件存在且能被 `mod.rs` 引用，编译通过"
        ],
        "passes": true
    },
    {
        "category": "backend",
        "description": "Task 1.3: 实现 Service Account 解析",
        "steps": [
            "编写 `parse_service_account(json_str: &str) -> Result<(String, String)>`",
            "逻辑: 解析 JSON，提取 `private_key` 和 `client_email`",
            "验证: 单元测试能正确解析 mock 的 json 字符串，提取 private_key 和 email"
        ],
        "passes": true
    },
    {
        "category": "backend",
        "description": "Task 1.4: 实现 OAuth Token 获取 (JWT 签名)",
        "steps": [
            "实现 `get_access_token(client_email, private_key) -> String`",
            "逻辑: 构造 JWT Claims (iss, scope, aud, exp, iat)，使用 RS256 签名，POST 请求 Google OAuth 端点",
            "优化: 实现简单的内存缓存 (RwLock<HashMap>) 以避免频繁请求 Token",
            "验证: 在 Mock Server 环境下，能正确发送 JWT 并解析返回的 Access Token"
        ],
        "passes": true
    },
    {
        "category": "backend",
        "description": "Task 1.5: 实现 Vertex 请求构造 (Build Request)",
        "steps": [
            "实现 `build_request`",
            "逻辑: 调用 `get_access_token` 获取 Token",
            "Header 添加 `Authorization: Bearer <token>`",
            "构造 URL: `https://<region>-aiplatform.googleapis.com/v1/projects/<project_id>/locations/<region>/publishers/google/models/<model>:streamGenerateContent`",
            "验证: 构造的 URL 和 Header 符合 Vertex AI 规范"
        ],
        "passes": true
    },
    {
        "category": "backend",
        "description": "Task 1.6: 复用与适配 Gemini 协议转换",
        "steps": [
            "在 `VertexAdaptor` 中复用 `GeminiAdaptor` 的 `convert_request`",
            "确保 Vertex AI 的 API 路径参数 (project_id, region) 能从 Upstream 配置或 Extra 参数中获取",
            "验证: 输入 OpenAI Request，输出符合 Vertex AI 的 JSON Body"
        ],
        "passes": false
    },
    {
        "category": "backend",
        "description": "Task 1.7: 注册 Vertex 适配器",
        "steps": [
            "在 `crates/router/src/adaptor/factory.rs` 中注册 `ChannelType::VertexAi`",
            "验证: `AdaptorFactory::get_adaptor(ChannelType::VertexAi)` 返回正确的适配器实例"
        ],
        "passes": false
    },
    {
        "category": "backend",
        "description": "Task 2.1: 为 ChannelAdaptor Trait 增加流式支持",
        "steps": [
            "修改 `ChannelAdaptor` trait，增加 `fn convert_stream_response(...)`",
            "验证: 编译通过"
        ],
        "passes": false
    },
    {
        "category": "backend",
        "description": "Task 2.2: 实现 Vertex/Gemini 流式转换",
        "steps": [
            "在 `VertexAdaptor` (及 `GeminiAdaptor`) 中实现 `convert_stream_response`",
            "逻辑: 解析 Google 的流式 JSON Array chunk (`[{candidates: ...}]`)",
            "转换: 输出 OpenAI SSE 格式 (`data: {...}`)",
            "验证: 单元测试，输入 Google chunk，输出 OpenAI SSE string"
        ],
        "passes": false
    },
    {
        "category": "backend",
        "description": "Task 2.3: 更新 Router Proxy Logic 支持流式",
        "steps": [
            "修改 `crates/router/src/lib.rs` 中的 `proxy_logic`",
            "逻辑: 当 `stream=true` 且 ChannelType 为 VertexAi/Gemini 时，使用 `convert_stream_response` 处理响应流",
            "验证: 编译通过"
        ],
        "passes": false
    },
    {
        "category": "integration",
        "description": "Task 3.1: 完整 Vertex AI 流程测试",
        "steps": [
            "在 UI 或 CLI 中添加一个 Google Vertex AI 渠道 (配置 Mock 的 Service Account)",
            "在 Playground 中选择该模型，发送对话请求 (Stream & Non-Stream)",
            "验证: 能成功收到回复，且无报错"
        ],
        "passes": false
    }
]