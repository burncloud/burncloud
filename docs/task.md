[
    {
        "category": "phase1-channel-state",
        "description": "Task 1.1.1: 创建 ChannelStateTracker 模块文件",
        "passes": true,
        "steps": [
            "文件: crates/router/src/channel_state.rs (新建)",
            "创建模块基础结构",
            "添加必要的 use 声明",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-channel-state",
        "description": "Task 1.1.2: 定义 BalanceStatus 枚举",
        "passes": true,
        "steps": [
            "文件: crates/router/src/channel_state.rs",
            "定义枚举: Ok, Low, Exhausted, Unknown",
            "添加 #[derive(Debug, Clone, PartialEq)]",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-channel-state",
        "description": "Task 1.1.3: 定义 ModelStatus 枚举",
        "passes": true,
        "steps": [
            "文件: crates/router/src/channel_state.rs",
            "定义枚举: Available, RateLimited, QuotaExhausted, ModelNotFound, TemporarilyDown",
            "添加 #[derive(Debug, Clone, PartialEq)]",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-channel-state",
        "description": "Task 1.1.4: 定义 ModelState 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/channel_state.rs",
            "定义字段: model, channel_id, status, rate_limit_until, last_error, last_error_time",
            "定义统计字段: success_count, failure_count, avg_latency_ms",
            "添加 adaptive_limit 字段",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-channel-state",
        "description": "Task 1.1.5: 定义 ChannelState 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/channel_state.rs",
            "定义渠道级字段: channel_id, auth_ok, balance_status, account_rate_limit_until",
            "定义模型状态 Map: models: DashMap<String, ModelState>",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-channel-state",
        "description": "Task 1.1.6: 实现 ChannelStateTracker 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/channel_state.rs",
            "定义字段: channel_states: DashMap<i32, ChannelState>",
            "实现 new() 构造函数",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-channel-state",
        "description": "Task 1.1.7: 实现 is_available 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/channel_state.rs",
            "检查渠道级状态: auth_ok, balance_status, account_rate_limit_until",
            "检查模型级状态: model_status, rate_limit_until",
            "返回 bool 表示是否可用",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-channel-state",
        "description": "Task 1.1.8: 实现 record_error 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/channel_state.rs",
            "根据 FailureType 更新渠道级或模型级状态",
            "记录错误信息和时间",
            "更新失败计数",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-channel-state",
        "description": "Task 1.1.9: 实现 record_success 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/channel_state.rs",
            "更新成功计数",
            "更新平均延迟",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-channel-state",
        "description": "Task 1.1.10: 实现 get_available_channels 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/channel_state.rs",
            "接收候选渠道列表",
            "过滤不可用渠道",
            "返回可用渠道列表",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-circuit-breaker",
        "description": "Task 1.2.1: 定义 FailureType 枚举",
        "passes": true,
        "steps": [
            "文件: crates/router/src/circuit_breaker.rs",
            "定义枚举变体: AuthFailed, PaymentRequired, RateLimited{scope, retry_after}, ModelNotFound, ServerError, Timeout",
            "添加 #[derive(Debug, Clone)]",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-circuit-breaker",
        "description": "Task 1.2.2: 定义 RateLimitScope 枚举",
        "passes": true,
        "steps": [
            "文件: crates/router/src/circuit_breaker.rs",
            "定义枚举: Account, Model, Unknown",
            "用于区分账户级和模型级限流",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-circuit-breaker",
        "description": "Task 1.2.3: 扩展 UpstreamState 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/circuit_breaker.rs",
            "添加 failure_type: Option<FailureType> 字段",
            "添加 rate_limit_until: Option<Instant> 字段",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-circuit-breaker",
        "description": "Task 1.2.4: 重构 record_failure 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/circuit_breaker.rs",
            "添加 failure_type 参数",
            "根据错误类型决定熔断行为",
            "账户级错误触发渠道级熔断",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-circuit-breaker",
        "description": "Task 1.2.5: 实现 get_failure_type 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/circuit_breaker.rs",
            "根据渠道ID获取最近的失败类型",
            "返回 Option<FailureType>",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-response-parser",
        "description": "Task 1.3.1: 创建 response_parser 模块文件",
        "passes": true,
        "steps": [
            "文件: crates/router/src/response_parser.rs (新建)",
            "创建模块基础结构",
            "添加必要的 use 声明 (http, serde_json 等)",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-response-parser",
        "description": "Task 1.3.2: 定义 RateLimitInfo 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/response_parser.rs",
            "定义字段: request_limit, token_limit, remaining, reset, retry_after, scope",
            "添加 #[derive(Debug, Clone, Default)]",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-response-parser",
        "description": "Task 1.3.3: 定义 ErrorInfo 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/response_parser.rs",
            "定义字段: error_type, message, code, scope",
            "用于存储解析后的错误信息",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-response-parser",
        "description": "Task 1.3.4: 实现 parse_rate_limit_info 统一入口",
        "passes": true,
        "steps": [
            "文件: crates/router/src/response_parser.rs",
            "接收 headers, body, channel_type 参数",
            "根据 channel_type 分发到具体解析函数",
            "返回 RateLimitInfo",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-response-parser",
        "description": "Task 1.3.5: 实现 parse_openai_rate_limit 函数",
        "passes": true,
        "steps": [
            "文件: crates/router/src/response_parser.rs",
            "解析 X-RateLimit-Limit-Requests 头",
            "解析 X-RateLimit-Limit-Tokens 头",
            "解析 Retry-After 头",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase1-response-parser",
        "description": "Task 1.3.6: 实现 parse_anthropic_rate_limit 函数",
        "passes": true,
        "steps": [
            "文件: crates/router/src/response_parser.rs",
            "解析 anthropic-ratelimit-requests-limit 头",
            "解析 anthropic-ratelimit-requests-reset 头",
            "解析 retry-after 头",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase1-response-parser",
        "description": "Task 1.3.7: 实现 parse_azure_rate_limit 函数",
        "passes": true,
        "steps": [
            "文件: crates/router/src/response_parser.rs",
            "解析 X-RateLimit-Limit 头",
            "解析 X-RateLimit-Remaining 头",
            "解析 X-RateLimit-Reset 头 (Unix 时间戳)",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase1-response-parser",
        "description": "Task 1.3.8: 实现 parse_gemini_rate_limit 函数",
        "passes": true,
        "steps": [
            "文件: crates/router/src/response_parser.rs",
            "从响应体解析 RESOURCE_EXHAUSTED 错误",
            "提取错误信息",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase1-response-parser",
        "description": "Task 1.3.9: 实现 parse_rate_limit_scope 函数",
        "passes": true,
        "steps": [
            "文件: crates/router/src/response_parser.rs",
            "根据响应体内容判断限流范围",
            "检测 'account', 'API key', 'model' 关键词",
            "返回 RateLimitScope 枚举",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase1-response-parser",
        "description": "Task 1.3.10: 实现 parse_error_response 函数",
        "passes": true,
        "steps": [
            "文件: crates/router/src/response_parser.rs",
            "根据 channel_type 解析不同供应商的错误格式",
            "提取 error_type, message, code",
            "返回 ErrorInfo",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-model-router",
        "description": "Task 1.4.1: 添加 ChannelStateTracker 到 RouterState",
        "passes": true,
        "steps": [
            "文件: crates/router/src/lib.rs",
            "在 RouterState 结构体中添加 channel_state_tracker 字段",
            "在 new() 中初始化 ChannelStateTracker",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-model-router",
        "description": "Task 1.4.2: 改进 route 方法集成状态过滤",
        "passes": true,
        "steps": [
            "文件: crates/router/src/model_router.rs",
            "在查询 abilities 后调用 get_available_channels",
            "过滤掉不可用的渠道",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-model-router",
        "description": "Task 1.4.3: 实现按健康分数排序",
        "passes": true,
        "steps": [
            "文件: crates/router/src/model_router.rs",
            "调用 sort_by_health 方法",
            "优先选择健康分数高的渠道",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-model-router",
        "description": "Task 1.4.4: 实现无可用渠道时返回错误",
        "passes": true,
        "steps": [
            "文件: crates/router/src/model_router.rs",
            "当所有渠道不可用时返回明确的错误",
            "错误信息包含模型名称和原因",
            "禁止降级到其他模型",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-router-integration",
        "description": "Task 1.5.1: 在 lib.rs 中解析成功响应头",
        "passes": true,
        "steps": [
            "文件: crates/router/src/lib.rs",
            "在请求成功后调用 parse_rate_limit_info",
            "调用 adaptive_limit.on_success",
            "调用 channel_state_tracker.record_success",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-router-integration",
        "description": "Task 1.5.2: 在 lib.rs 中解析失败响应",
        "passes": true,
        "steps": [
            "文件: crates/router/src/lib.rs",
            "在请求失败后调用 parse_error_response",
            "根据错误类型调用 circuit_breaker.record_failure",
            "调用 channel_state_tracker.record_error",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-router-integration",
        "description": "Task 1.5.3: 处理 429 响应的特殊逻辑",
        "passes": true,
        "steps": [
            "文件: crates/router/src/lib.rs",
            "检测 429 状态码",
            "解析限流信息",
            "更新渠道/模型限流状态",
            "考虑重试下一个渠道",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-router-integration",
        "description": "Task 1.5.4: 处理 401/402 错误的渠道级禁用",
        "passes": true,
        "steps": [
            "文件: crates/router/src/lib.rs",
            "检测 401 (认证失败) 状态码",
            "检测 402 (余额不足) 状态码",
            "设置 channel_state.auth_ok = false 或 balance_status = Exhausted",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase1-router-integration",
        "description": "Task 1.5.5: 处理 404 模型不存在错误",
        "passes": true,
        "steps": [
            "文件: crates/router/src/lib.rs",
            "检测 404 状态码",
            "设置 model_state.status = ModelNotFound",
            "考虑从 abilities 移除该模型",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-adaptive-limit",
        "description": "Task 2.1.1: 创建 adaptive_limit 模块文件",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptive_limit.rs (新建)",
            "创建模块基础结构",
            "添加必要的 use 声明",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-adaptive-limit",
        "description": "Task 2.1.2: 定义 RateLimitState 枚举",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptive_limit.rs",
            "定义枚举: Learning, Stable, Cooldown",
            "添加 #[derive(Debug, Clone, PartialEq)]",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-adaptive-limit",
        "description": "Task 2.1.3: 定义 AdaptiveLimitConfig 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptive_limit.rs",
            "定义配置字段: learning_duration, initial_limit, adjustment_step, success_threshold, failure_threshold, cooldown_duration, recovery_ratio, max_limit",
            "实现 Default trait",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-adaptive-limit",
        "description": "Task 2.1.4: 定义 AdaptiveRateLimit 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptive_limit.rs",
            "定义字段: learned_limit, current_limit, state, success_streak, failure_streak, cooldown_until, rate_limit_until, last_adjusted_at",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-adaptive-limit",
        "description": "Task 2.1.5: 实现 AdaptiveRateLimit::new 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptive_limit.rs",
            "接收 config 参数",
            "初始化各字段默认值",
            "state 初始为 Learning",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-adaptive-limit",
        "description": "Task 2.1.6: 实现 on_success 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptive_limit.rs",
            "从响应头学习上游限制",
            "更新连续成功计数",
            "实现 LEARNING → STABLE 状态转换",
            "在 LEARNING 状态下尝试提升限制",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase2-adaptive-limit",
        "description": "Task 2.1.7: 实现 on_rate_limited 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptive_limit.rs",
            "更新失败计数",
            "降低当前限制到 80%",
            "检查是否进入 COOLDOWN 状态",
            "记录限流解除时间",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase2-adaptive-limit",
        "description": "Task 2.1.8: 实现 check_available 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptive_limit.rs",
            "检查 COOLDOWN 状态",
            "检查限流时间是否已过",
            "返回 bool 表示是否可用",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase2-adaptive-limit",
        "description": "Task 2.1.9: 实现 recover_from_cooldown 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptive_limit.rs",
            "将状态设置为 Learning",
            "将限制降低到 50%",
            "重置失败计数",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase2-price-sync",
        "description": "Task 2.2.1: 创建 price_sync 模块文件",
        "passes": true,
        "steps": [
            "文件: crates/router/src/price_sync.rs (新建)",
            "创建模块基础结构",
            "添加必要的 use 声明 (reqwest, serde, tokio)",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-price-sync",
        "description": "Task 2.2.2: 定义 LiteLLM 价格数据结构",
        "passes": true,
        "steps": [
            "文件: crates/router/src/price_sync.rs",
            "定义 LiteLLMPrice 结构体: model, input_price, output_price, context_window, max_output_tokens",
            "添加 serde 反序列化属性",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-price-sync",
        "description": "Task 2.2.3: 定义 PriceSyncService 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/price_sync.rs",
            "定义字段: db, http_client, sync_interval",
            "实现 new() 构造函数",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-price-sync",
        "description": "Task 2.2.4: 实现 fetch_litellm_prices 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/price_sync.rs",
            "从 GitHub 获取 JSON 数据",
            "URL: https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json",
            "解析 JSON 为 HashMap<String, LiteLLMPrice>",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase2-price-sync",
        "description": "Task 2.2.5: 实现 sync_from_litellm 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/price_sync.rs",
            "调用 fetch_litellm_prices",
            "遍历价格数据并更新 prices 表",
            "返回更新的记录数",
            "验证: 集成测试通过"
        ]
    },
    {
        "category": "phase2-price-sync",
        "description": "Task 2.2.6: 实现定时同步任务启动",
        "passes": true,
        "steps": [
            "文件: crates/router/src/lib.rs",
            "在 Router 启动时创建 PriceSyncService",
            "使用 tokio::spawn 启动后台同步任务",
            "每小时执行一次同步",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-price-sync",
        "description": "Task 2.2.7: 扩展 prices 表添加同步字段",
        "passes": true,
        "steps": [
            "文件: crates/database/src/schema.rs",
            "添加 synced_at 字段 (DATETIME)",
            "添加 source 字段 (VARCHAR: litellm/manual)",
            "创建迁移脚本",
            "验证: 数据库迁移成功"
        ]
    },
    {
        "category": "phase2-price-sync",
        "description": "Task 2.2.8: 实现 PriceModel::upsert 方法",
        "passes": true,
        "steps": [
            "文件: crates/database-models/src/lib.rs",
            "实现插入或更新价格记录",
            "更新 synced_at 时间戳",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase2-notification",
        "description": "Task 2.3.1: 创建 notification 模块文件",
        "passes": true,
        "steps": [
            "文件: crates/router/src/notification.rs (新建)",
            "创建模块基础结构",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-notification",
        "description": "Task 2.3.2: 定义 NotificationService 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/notification.rs",
            "定义通知配置字段",
            "实现 new() 构造函数",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-notification",
        "description": "Task 2.3.3: 实现 notify_new_model 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/notification.rs",
            "发送新模型发现通知",
            "包含模型名称和首次请求时间",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-notification",
        "description": "Task 2.3.4: 实现 notify_price_missing 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/notification.rs",
            "发送价格缺失通知",
            "提示管理员配置价格",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase2-notification",
        "description": "Task 2.3.5: 实现 notify_channel_error 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/notification.rs",
            "发送渠道错误通知",
            "包含错误类型和渠道信息",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-protocol-config",
        "description": "Task 3.1.1: 创建 protocol_configs 表",
        "passes": true,
        "steps": [
            "文件: crates/database/src/schema.rs",
            "定义表结构: id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint, models_endpoint, request_mapping, response_mapping, detection_rules, created_at, updated_at",
            "添加 UNIQUE(channel_type, api_version) 约束",
            "验证: 数据库迁移成功"
        ]
    },
    {
        "category": "phase3-protocol-config",
        "description": "Task 3.1.2: 定义 ProtocolConfig 结构体",
        "passes": true,
        "steps": [
            "文件: crates/common/src/types.rs",
            "定义字段对应数据库表结构",
            "添加 #[derive(Debug, Clone, Deserialize, Serialize)]",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-protocol-config",
        "description": "Task 3.1.3: 实现 ProtocolConfigModel CRUD",
        "passes": true,
        "steps": [
            "文件: crates/database-models/src/lib.rs",
            "实现 get_by_type_version 方法",
            "实现 get_default 方法",
            "实现 create/update/delete 方法",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase3-protocol-config",
        "description": "Task 3.1.4: 定义 RequestMapping 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/mapping.rs (新建)",
            "定义字段: field_map, rename, add_fields",
            "添加 serde 反序列化支持",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-protocol-config",
        "description": "Task 3.1.5: 定义 ResponseMapping 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/mapping.rs",
            "定义字段: content_path, usage_path, error_path",
            "添加 serde 反序列化支持",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-protocol-config",
        "description": "Task 3.1.6: 实现 apply_mapping 函数",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/mapping.rs",
            "应用字段映射规则到 JSON",
            "处理字段重命名",
            "添加固定字段",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase3-protocol-config",
        "description": "Task 3.1.7: 实现 extract_value 函数",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/mapping.rs",
            "支持 JSONPath 语法提取值",
            "支持数组索引访问 (如 choices[0])",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase3-dynamic-adaptor",
        "description": "Task 3.2.1: 创建 dynamic.rs 适配器文件",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/dynamic.rs (新建)",
            "创建模块基础结构",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-dynamic-adaptor",
        "description": "Task 3.2.2: 定义 DynamicAdaptor 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/dynamic.rs",
            "定义字段: config (ProtocolConfig), channel_type",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-dynamic-adaptor",
        "description": "Task 3.2.3: 实现 Adaptor trait for DynamicAdaptor",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/dynamic.rs",
            "实现 prepare_request 方法",
            "实现 parse_response 方法",
            "实现其他必要方法",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-dynamic-adaptor",
        "description": "Task 3.2.4: 实现 prepare_request 动态路径构建",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/dynamic.rs",
            "替换路径中的 {deployment_id} 占位符",
            "应用请求映射规则",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase3-dynamic-adaptor",
        "description": "Task 3.2.5: 实现 parse_response 动态解析",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/dynamic.rs",
            "应用响应映射规则",
            "提取 content 和 usage",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase3-dynamic-adaptor",
        "description": "Task 3.2.6: 创建 DynamicAdaptorFactory",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/factory.rs",
            "定义结构体包含 db 和 cache",
            "实现 get_adaptor 方法",
            "实现配置缓存逻辑",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-dynamic-adaptor",
        "description": "Task 3.2.7: 集成 DynamicAdaptorFactory 到路由",
        "passes": true,
        "steps": [
            "文件: crates/router/src/lib.rs",
            "在 RouterState 中添加 adaptor_factory",
            "在路由逻辑中使用 factory 获取适配器",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-api-detector",
        "description": "Task 3.3.1: 创建 detector.rs 文件",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/detector.rs (新建)",
            "创建模块基础结构",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-api-detector",
        "description": "Task 3.3.2: 定义 ApiVersionDetector 结构体",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/detector.rs",
            "定义字段: db",
            "实现 new() 构造函数",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-api-detector",
        "description": "Task 3.3.3: 实现 detect_and_update 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/detector.rs",
            "解析弃用错误信息",
            "更新渠道的 api_version",
            "清除适配器缓存",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase3-api-detector",
        "description": "Task 3.3.4: 实现 parse_deprecation_error 方法",
        "passes": true,
        "steps": [
            "文件: crates/router/src/adaptor/detector.rs",
            "使用正则表达式匹配新版本号",
            "支持 Azure/OpenAI 格式",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase3-api-detector",
        "description": "Task 3.3.5: 集成检测器到错误处理流程",
        "passes": true,
        "steps": [
            "文件: crates/router/src/lib.rs",
            "在特定错误响应时调用 detector",
            "处理检测结果",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-cli-protocol",
        "description": "Task 3.4.1: 创建 protocol.rs CLI 模块",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/protocol.rs (新建)",
            "创建模块基础结构",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase3-cli-protocol",
        "description": "Task 3.4.2: 实现 protocol list 命令",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/protocol.rs",
            "查询 protocol_configs 表",
            "表格显示: ID, ChannelType, API Version, Default, Endpoint",
            "验证: 手动测试通过"
        ]
    },
    {
        "category": "phase3-cli-protocol",
        "description": "Task 3.4.3: 实现 protocol add 命令",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/protocol.rs",
            "必需参数: --channel-type, --api-version",
            "可选参数: --chat-endpoint, --request-mapping, --response-mapping",
            "插入数据库",
            "验证: 手动测试通过"
        ]
    },
    {
        "category": "phase3-cli-protocol",
        "description": "Task 3.4.4: 实现 protocol test 命令",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/protocol.rs",
            "参数: --channel-id, --model",
            "测试协议配置是否有效",
            "输出测试结果",
            "验证: 手动测试通过"
        ]
    },
    {
        "category": "phase3-cli-protocol",
        "description": "Task 3.4.5: 在 commands.rs 中添加 protocol 子命令",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/commands.rs",
            "添加 protocol 子命令定义",
            "添加处理逻辑",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "phase4-channel-cli",
        "description": "Task 4.1.1: 修改 channel add 命令自动创建 abilities",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/channel.rs",
            "解析 -m/--models 参数获取模型列表",
            "创建渠道后自动创建 abilities 记录",
            "使用 'default' 作为 group",
            "验证: 手动测试通过"
        ]
    },
    {
        "category": "phase4-channel-cli",
        "description": "Task 4.1.2: 添加 Channel.api_version 字段支持",
        "passes": true,
        "steps": [
            "文件: crates/common/src/types.rs",
            "在 Channel 结构体添加 api_version 字段",
            "默认值为 'default'",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "phase4-channel-cli",
        "description": "Task 4.1.3: 更新 channels 表添加 api_version 列",
        "passes": true,
        "steps": [
            "文件: crates/database/src/schema.rs",
            "添加 api_version 列 (VARCHAR, DEFAULT 'default')",
            "创建迁移脚本",
            "验证: 数据库迁移成功"
        ]
    },
    {
        "category": "phase4-channel-cli",
        "description": "Task 4.1.4: 实现 AbilityModel::create_batch 方法",
        "passes": false,
        "steps": [
            "文件: crates/database-models/src/lib.rs",
            "批量创建 abilities 记录",
            "处理已存在的记录",
            "验证: 单元测试通过"
        ]
    },
    {
        "category": "phase4-model-capabilities",
        "description": "Task 4.2.1: 创建 model_capabilities 表",
        "passes": false,
        "steps": [
            "文件: crates/database/src/schema.rs",
            "定义表结构: model, context_window, max_output_tokens, supports_vision, supports_function_calling, input_price, output_price, synced_at",
            "验证: 数据库迁移成功"
        ]
    },
    {
        "category": "phase4-model-capabilities",
        "description": "Task 4.2.2: 扩展 PriceSyncService 同步能力数据",
        "passes": false,
        "steps": [
            "文件: crates/router/src/price_sync.rs",
            "解析 LiteLLM 数据中的能力字段",
            "更新 model_capabilities 表",
            "验证: 集成测试通过"
        ]
    },
    {
        "category": "phase5-tests",
        "description": "Task 5.1.1: 编写状态机转换单元测试",
        "passes": false,
        "steps": [
            "文件: crates/router/src/adaptive_limit.rs",
            "测试 LEARNING → STABLE 转换",
            "测试 → COOLDOWN 转换",
            "测试 COOLDOWN → LEARNING 恢复",
            "验证: cargo test 通过"
        ]
    },
    {
        "category": "phase5-tests",
        "description": "Task 5.1.2: 编写响应头解析单元测试",
        "passes": false,
        "steps": [
            "文件: crates/router/src/response_parser.rs",
            "测试 OpenAI 格式解析",
            "测试 Anthropic 格式解析",
            "测试 Azure 格式解析",
            "测试 Gemini 格式解析",
            "验证: cargo test 通过"
        ]
    },
    {
        "category": "phase5-tests",
        "description": "Task 5.1.3: 编写渠道选择集成测试",
        "passes": false,
        "steps": [
            "文件: crates/tests/integration_test.rs (或新建)",
            "测试健康渠道优先选择",
            "测试限流渠道过滤",
            "测试无可用渠道错误返回",
            "验证: cargo test 通过"
        ]
    },
    {
        "category": "phase5-tests",
        "description": "Task 5.1.4: 编写价格同步集成测试",
        "passes": false,
        "steps": [
            "文件: crates/tests/integration_test.rs",
            "模拟 LiteLLM 数据源",
            "测试价格更新流程",
            "测试能力数据同步",
            "验证: cargo test 通过"
        ]
    },
    {
        "category": "phase5-tests",
        "description": "Task 5.1.5: 编写协议适配集成测试",
        "passes": false,
        "steps": [
            "文件: crates/tests/integration_test.rs",
            "测试动态路径替换",
            "测试请求映射",
            "测试响应映射",
            "验证: cargo test 通过"
        ]
    },
    {
        "category": "phase6-docs",
        "description": "Task 6.1.1: 更新 docs/plan.md 整合最终方案",
        "passes": false,
        "steps": [
            "文件: docs/plan.md",
            "确保包含价格自动同步章节",
            "确保包含自适应限流章节",
            "确保包含可配置协议适配器章节",
            "验证: 文档完整"
        ]
    },
    {
        "category": "phase6-docs",
        "description": "Task 6.1.2: 更新 CLAUDE.md 添加新模块说明",
        "passes": false,
        "steps": [
            "文件: CLAUDE.md",
            "添加 channel_state.rs 说明",
            "添加 adaptive_limit.rs 说明",
            "添加 response_parser.rs 说明",
            "添加 price_sync.rs 说明",
            "验证: 文档完整"
        ]
    }
]
