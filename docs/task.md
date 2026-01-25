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
        "passes": false
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
        "passes": false
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
        "passes": false
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
        "passes": false
    }
]