[
    {
        "category": "token-stats",
        "description": "Task 1.1: 分析各供应商流式响应中的token统计格式",
        "passes": true,
        "steps": [
            "研究OpenAI流式响应: data: {\"choices\":[{\"delta\":...}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":20}}",
            "研究Anthropic流式响应: data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":15}}",
            "研究Gemini流式响应: metadata.usageMetadata",
            "记录每种格式的解析要点"
        ]
    },
    {
        "category": "token-stats",
        "description": "Task 1.2: 定义流式Token统计结构体",
        "passes": true,
        "steps": [
            "在 crates/router/src/lib.rs 或新模块中定义 StreamingTokenCounter 结构体",
            "字段: prompt_tokens: AtomicU32, completion_tokens: AtomicU32",
            "方法: increment_completion(n), get_usage()",
            "验证: cargo check -p burncloud-router 编译通过"
        ]
    },
    {
        "category": "token-stats",
        "description": "Task 1.3: 实现OpenAI流式响应token解析",
        "passes": true,
        "steps": [
            "在流式响应处理中监听 data: [DONE] 之前的usage字段",
            "解析 usage.prompt_tokens 和 usage.completion_tokens",
            "更新 StreamingTokenCounter",
            "验证: 单元测试覆盖解析逻辑"
        ]
    },
    {
        "category": "token-stats",
        "description": "Task 1.4: 实现Anthropic流式响应token解析",
        "passes": true,
        "steps": [
            "监听 message_start 事件获取 input_tokens",
            "监听 message_delta 事件获取 output_tokens",
            "累加到 StreamingTokenCounter",
            "验证: 单元测试覆盖解析逻辑"
        ]
    },
    {
        "category": "token-stats",
        "description": "Task 1.5: 实现Gemini流式响应token解析",
        "passes": true,
        "steps": [
            "监听 usageMetadata 字段",
            "解析 promptTokenCount 和 candidatesTokenCount",
            "更新 StreamingTokenCounter",
            "验证: 单元测试覆盖解析逻辑"
        ]
    },
    {
        "category": "token-stats",
        "description": "Task 1.6: 将token统计写入router_logs表",
        "passes": true,
        "steps": [
            "修改 proxy_logic() 在请求结束时获取token计数",
            "将 completion_tokens 写入 DbRouterLog",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "token-stats",
        "description": "Task 1.7: 集成测试 - 流式请求token统计",
        "passes": true,
        "steps": [
            "启动服务器",
            "发送流式请求到 OpenAI/Claude/Gemini 渠道",
            "检查 router_logs 表中 prompt_tokens 和 completion_tokens 不为0",
            "验证统计值与上游返回一致"
        ]
    },
    {
        "category": "token-expiry",
        "description": "Task 2.1: 分析当前validate_token函数",
        "passes": true,
        "steps": [
            "阅读 crates/database/crates/database-router/src/lib.rs 中 validate_token 函数",
            "确认 expired_time 字段是否存在及其语义 (-1=永不过期, >0=过期时间戳)",
            "记录当前验证逻辑"
        ]
    },
    {
        "category": "token-expiry",
        "description": "Task 2.2: 实现过期时间检查逻辑",
        "passes": true,
        "steps": [
            "在 validate_token 函数中添加过期检查",
            "if token.expired_time > 0 && token.expired_time < now() { return TokenExpired }",
            "定义 TokenExpired 错误类型",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "token-expiry",
        "description": "Task 2.3: 定义Token过期错误响应",
        "passes": true,
        "steps": [
            "定义错误码: TOKEN_EXPIRED = 401",
            "定义错误消息: \"Token has expired\"",
            "确保错误响应符合API格式规范",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "token-expiry",
        "description": "Task 2.4: 集成测试 - 过期Token验证",
        "passes": true,
        "steps": [
            "创建一个过期时间设置为过去时间的Token",
            "使用该Token发送请求",
            "验证返回 401 错误",
            "验证错误消息正确"
        ]
    },
    {
        "category": "pricing",
        "description": "Task 3.1: 设计prices表结构",
        "passes": true,
        "steps": [
            "设计表字段: id, model, input_price, output_price, currency, created_at, updated_at",
            "考虑模型别名映射 (gpt-4-turbo -> gpt-4 定价)",
            "在 docs/schema.sql 中记录设计"
        ]
    },
    {
        "category": "pricing",
        "description": "Task 3.2: 添加prices表到schema.rs",
        "passes": true,
        "steps": [
            "打开 crates/database/src/schema.rs",
            "添加 prices 表定义",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "pricing",
        "description": "Task 3.3: 创建Price数据模型",
        "passes": true,
        "steps": [
            "在 crates/database-models/src/lib.rs 或新文件定义 Price 结构体",
            "实现 FromRow trait",
            "实现 CRUD 方法: PriceModel::create, get, list, update, upsert",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "pricing",
        "description": "Task 3.4: 创建数据库迁移脚本",
        "passes": true,
        "steps": [
            "创建 migrations/xxx_create_prices.sql 文件",
            "写入 CREATE TABLE IF NOT EXISTS prices 语句",
            "添加默认定价数据 (gpt-4, claude-3, gemini-1.5 等)",
            "验证: SQL语法正确"
        ]
    },
    {
        "category": "pricing",
        "description": "Task 3.5: 实现PricingService服务层",
        "passes": true,
        "steps": [
            "创建 crates/service/crates/service-pricing/src/lib.rs",
            "实现 calculate_cost(model, prompt_tokens, completion_tokens) -> f64",
            "实现 get_price(model) -> Option<Price>",
            "处理模型别名映射",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "pricing",
        "description": "Task 3.6: 集成计费逻辑到router",
        "passes": false,
        "steps": [
            "在 proxy_logic() 请求结束后调用 PricingService::calculate_cost()",
            "将费用写入 router_logs 或新表",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "pricing",
        "description": "Task 3.7: 实现CLI定价管理命令",
        "passes": false,
        "steps": [
            "添加 burncloud price list 命令",
            "添加 burncloud price set <model> --input <price> --output <price> 命令",
            "验证: cargo run -- price --help 显示帮助"
        ]
    },
    {
        "category": "pricing",
        "description": "Task 3.8: 集成测试 - 计费逻辑",
        "passes": false,
        "steps": [
            "设置模型定价: gpt-4 input=$0.03/1k, output=$0.06/1k",
            "发送请求消耗 100 prompt + 200 completion tokens",
            "验证计算费用: 100*0.03/1000 + 200*0.06/1000 = $0.015",
            "验证费用正确记录"
        ]
    },
    {
        "category": "quota",
        "description": "Task 4.1: 分析当前配额数据结构",
        "passes": false,
        "steps": [
            "阅读 users 表: quota, used_quota 字段",
            "阅读 tokens 表: remain_quota, used_quota 字段",
            "理解 unlimited_quota 语义",
            "记录配额继承关系"
        ]
    },
    {
        "category": "quota",
        "description": "Task 4.2: 实现配额扣除服务",
        "passes": false,
        "steps": [
            "创建 crates/service/crates/service-user/src/quota.rs 或使用现有模块",
            "实现 deduct_quota(user_id, token_id, cost) -> Result<()>",
            "实现原子性扣费 (使用事务)",
            "处理配额不足情况: 返回 QuotaInsufficient 错误",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "quota",
        "description": "Task 4.3: 集成配额扣除到router",
        "passes": false,
        "steps": [
            "在请求开始时检查配额是否充足",
            "在请求结束后扣除配额",
            "处理扣费失败情况 (记录日志但不影响响应)",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "quota",
        "description": "Task 4.4: 定义配额不足错误响应",
        "passes": false,
        "steps": [
            "定义错误码: QUOTA_INSUFFICIENT = 402",
            "定义错误消息: \"Insufficient quota\"",
            "确保错误响应符合API格式规范",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "quota",
        "description": "Task 4.5: 集成测试 - 配额扣除",
        "passes": false,
        "steps": [
            "创建用户配额为 100 的用户",
            "发送请求消耗 50 配额",
            "验证用户 used_quota 变为 50",
            "发送请求消耗 60 配额",
            "验证返回 402 错误"
        ]
    },
    {
        "category": "token-access",
        "description": "Task 5.1: 实现Token访问时间更新",
        "passes": true,
        "steps": [
            "在 validate_token 函数中，验证成功后更新 accessed_time",
            "使用非阻塞方式更新 (spawn task 或直接更新)",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "token-access",
        "description": "Task 5.2: 集成测试 - 访问时间更新",
        "passes": true,
        "steps": [
            "创建Token，记录初始 accessed_time",
            "使用Token发送请求",
            "验证 accessed_time 已更新为当前时间"
        ]
    },
    {
        "category": "weighted-balance",
        "description": "Task 6.1: 设计WeightedBalancer结构",
        "passes": false,
        "steps": [
            "定义 WeightedBalancer 结构体",
            "字段: channels: Vec<(Channel, weight)>, total_weight: u32",
            "方法: select() -> Option<Channel>",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "weighted-balance",
        "description": "Task 6.2: 实现加权随机选择算法",
        "passes": false,
        "steps": [
            "实现 select() 方法",
            "生成随机数 r in [0, total_weight)",
            "遍历channels累加权重直到超过r，返回对应channel",
            "验证: 单元测试覆盖边界情况"
        ]
    },
    {
        "category": "weighted-balance",
        "description": "Task 6.3: 集成WeightedBalancer到ModelRouter",
        "passes": false,
        "steps": [
            "修改 ModelRouter::route() 方法",
            "根据 abilities 表中的 weight 字段使用 WeightedBalancer",
            "保持向后兼容 (weight=0 时使用 Round-Robin)",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "weighted-balance",
        "description": "Task 6.4: 集成测试 - 加权负载均衡",
        "passes": false,
        "steps": [
            "创建两个渠道: Channel A (weight=80), Channel B (weight=20)",
            "发送100个请求",
            "验证约80个请求到A，约20个到B",
            "误差在合理范围内 (±10%)"
        ]
    },
    {
        "category": "integration",
        "description": "Task 7.1: 端到端测试 - 完整计费流程",
        "passes": false,
        "steps": [
            "创建用户、Token、Channel、定价配置",
            "发送流式请求",
            "验证: token统计正确",
            "验证: 费用计算正确",
            "验证: 配额扣除正确",
            "验证: 访问时间更新"
        ]
    },
    {
        "category": "integration",
        "description": "Task 7.2: 性能测试 - 并发请求",
        "passes": false,
        "steps": [
            "使用工具 (wrk/ab) 发送100并发请求",
            "验证无数据竞争",
            "验证无死锁",
            "验证响应时间在可接受范围"
        ]
    },
    {
        "category": "integration",
        "description": "Task 7.3: 边界测试 - 异常情况",
        "passes": false,
        "steps": [
            "测试无效Token: 返回401",
            "测试过期Token: 返回401",
            "测试配额不足: 返回402",
            "测试模型不存在: 返回404",
            "测试上游不可用: 返回503"
        ]
    },
    {
        "category": "docs",
        "description": "Task 8.1: 更新API文档",
        "passes": false,
        "steps": [
            "更新 docs/api.md 添加新错误码说明",
            "添加计费API文档",
            "添加配额管理API文档",
            "验证: 文档格式正确"
        ]
    },
    {
        "category": "docs",
        "description": "Task 8.2: 更新README",
        "passes": false,
        "steps": [
            "更新 README.md 添加计费功能说明",
            "添加定价配置示例",
            "添加CLI使用示例",
            "验证: markdown格式正确"
        ]
    },
    {
        "category": "release",
        "description": "Task 9.1: 准备发布检查清单",
        "passes": false,
        "steps": [
            "运行 cargo clippy --all-targets --all-features",
            "运行 cargo fmt --all -- --check",
            "运行 cargo test --all-features",
            "修复所有警告和错误"
        ]
    },
    {
        "category": "release",
        "description": "Task 9.2: 准备数据库迁移说明",
        "passes": false,
        "steps": [
            "编写迁移步骤文档",
            "准备从旧版本升级的SQL脚本",
            "验证迁移脚本可重复执行"
        ]
    }
]
