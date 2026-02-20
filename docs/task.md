{
    "meta": {
        "title": "高级定价维度支持 - 实施任务清单",
        "version": "4.0",
        "source": "docs/plan.md 第十三、十四、十五章",
        "priority_order": [
            "P1",
            "P2",
            "P3"
        ],
        "estimated_phases": 9,
        "total_tasks": 40,
        "estimated_effort": "7-10 days",
        "task_status_field": {
            "passes": "null=未开始, true=已完成, false=失败需重试"
        }
    },
    "context": {
        "problem": "当前价格同步仅支持基础定价字段，无法处理缓存定价、批量定价、阶梯定价等高级场景，导致计费不准确（如 Qwen 阶梯定价误差可达 46%）",
        "goal": "扩展价格系统支持高级定价维度，实现精确计费",
        "constraints": [
            "保持向后兼容，现有简单计费逻辑继续工作",
            "阶梯计费为可选功能，不影响默认行为",
            "数据库迁移需支持 SQLite 和 PostgreSQL"
        ],
        "negative_constraints": [
            "不要在计费逻辑中使用 unwrap() 或 expect()",
            "不要在热路径（proxy_handler）中同步查询数据库",
            "不要在价格同步时锁定整个 prices 表",
            "不要删除现有价格记录，只能 upsert",
            "不要在计费失败时阻塞请求",
            "不要使用 f32 类型处理金额"
        ],
        "security": {
            "input_validation": "所有价格输入必须 >= 0",
            "sql_injection": "使用参数化查询，禁止字符串拼接 SQL",
            "access_control": "阶梯定价配置仅管理员可修改"
        },
        "metrics": {
            "billing_accuracy": "计费误差 < 0.01%",
            "sync_latency": "价格同步时间 < 30s",
            "query_latency": "阶梯定价查询 < 1ms"
        }
    },
    "execution_order": [
        [
            "1.1"
        ],
        [
            "1.2",
            "1.3"
        ],
        [
            "1.4"
        ],
        [
            "2.1",
            "2.2",
            "2.3"
        ],
        [
            "2.4"
        ],
        [
            "3.1",
            "3.2",
            "3.3",
            "3.4"
        ],
        [
            "4.1"
        ],
        [
            "4.2",
            "4.3",
            "5.1",
            "5.2"
        ],
        [
            "6.1",
            "6.2",
            "6.3"
        ],
        [
            "7.1",
            "7.2"
        ],
        [
            "8.1",
            "8.2"
        ],
        [
            "8.3"
        ],
        [
            "8.4",
            "8.5",
            "8.6",
            "8.7"
        ],
        [
            "8.8"
        ],
        [
            "9.1",
            "9.2"
        ],
        [
            "9.3",
            "9.4"
        ],
        [
            "9.5",
            "9.6"
        ],
        [
            "9.7",
            "9.8",
            "9.9"
        ]
    ],
    "tasks": [
        {
            "id": "1.1",
            "passes": true,
            "category": "phase1-database-schema",
            "priority": "P2",
            "description": "扩展 prices 表支持高级定价字段",
            "context": {
                "why": "当前 prices 表只有 input_price/output_price，无法存储缓存定价、批量定价等高级字段",
                "impact": "所有使用 prices 表的模块需适配新字段",
                "risk": "数据库迁移失败会影响整个系统启动"
            },
            "steps": [
                {
                    "action": "修改 schema.rs 添加字段定义",
                    "file": "crates/database/src/schema.rs",
                    "detail": "使用 diesel::allow_tables_to_appear_in_same_query! 宏确保兼容"
                },
                {
                    "action": "添加字段: cache_read_price REAL",
                    "default": "NULL",
                    "comment": "缓存命中价格，约为标准价格的 10%"
                },
                {
                    "action": "添加字段: cache_creation_price REAL",
                    "default": "NULL",
                    "comment": "缓存创建价格"
                },
                {
                    "action": "添加字段: batch_input_price REAL",
                    "default": "NULL",
                    "comment": "批量请求输入价格，约为标准的 50%"
                },
                {
                    "action": "添加字段: batch_output_price REAL",
                    "default": "NULL"
                },
                {
                    "action": "添加字段: priority_input_price REAL",
                    "default": "NULL",
                    "comment": "高优先级请求价格，约为标准的 170%"
                },
                {
                    "action": "添加字段: priority_output_price REAL",
                    "default": "NULL"
                },
                {
                    "action": "添加字段: audio_input_price REAL",
                    "default": "NULL",
                    "comment": "音频 token 价格，约为文本的 7 倍"
                },
                {
                    "action": "添加字段: full_pricing TEXT",
                    "default": "NULL",
                    "comment": "JSON blob 用于未来扩展字段"
                },
                {
                    "action": "创建迁移脚本",
                    "file": "migrations/YYYYMMDDHHMMSS_add_advanced_pricing_fields/up.sql",
                    "sqlite": "ALTER TABLE prices ADD COLUMN ...",
                    "postgresql": "ALTER TABLE prices ADD COLUMN ...;"
                }
            ],
            "acceptance_criteria": [
                "GIVEN 现有 prices 表有数据",
                "WHEN 执行迁移脚本",
                "THEN 所有新字段为 NULL，现有数据完整保留",
                "AND cargo build 编译通过",
                "AND 应用能正常启动"
            ],
            "rollback": {
                "sqlite": "不支持 DROP COLUMN，需重建表",
                "postgresql": "ALTER TABLE prices DROP COLUMN cache_read_price; ..."
            },
            "error_handling": [
                "迁移失败: 回滚事务，记录错误日志，应用退出",
                "字段已存在: 跳过该字段，继续其他字段"
            ],
            "verification": [
                "cargo build 编译通过",
                "数据库迁移成功执行",
                "现有数据不受影响",
                "新字段默认值为 NULL"
            ],
            "constraints": [
                "所有新字段默认 NULL，保持向后兼容",
                "迁移脚本放在 migrations/ 目录",
                "支持 SQLite 3.35+ 和 PostgreSQL 12+"
            ],
            "references": [
                "docs/plan.md 第十三章 - 方案 A",
                "crates/database/src/schema.rs - 现有 prices 表定义"
            ]
        },
        {
            "id": "1.2",
            "passes": true,
            "category": "phase1-database-schema",
            "priority": "P2",
            "description": "创建阶梯定价表 tiered_pricing",
            "context": "Qwen 等模型存在阶梯定价，输入 token 越长单价越高，当前简单公式会导致 46% 计费误差",
            "steps": [
                "文件: crates/database/src/schema.rs",
                "创建表 tiered_pricing",
                "字段: id INTEGER PRIMARY KEY",
                "字段: model TEXT NOT NULL",
                "字段: region TEXT (cn/international/NULL)",
                "字段: tier_start INTEGER NOT NULL (起始 tokens)",
                "字段: tier_end INTEGER NOT NULL (结束 tokens)",
                "字段: input_price REAL NOT NULL",
                "字段: output_price REAL NOT NULL",
                "约束: UNIQUE(model, region, tier_start)",
                "创建索引: idx_tiered_pricing_model"
            ],
            "verification": [
                "cargo build 编译通过",
                "数据库迁移成功",
                "能插入测试数据: Qwen 阶梯定价数据"
            ],
            "constraints": [
                "tier_end 可以为 NULL 表示无上限",
                "region 为 NULL 表示通用定价"
            ],
            "references": [
                "docs/plan.md 第十三章 - Qwen 阶梯定价方案 1",
                "Qwen 国内版: 0-32K $0.359/1M, 32K-128K $0.574/1M, 128K-252K $1.004/1M"
            ]
        },
        {
            "id": "1.3",
            "passes": true,
            "category": "phase1-data-models",
            "priority": "P2",
            "description": "定义高级定价数据结构",
            "context": "需要 Rust 结构体映射新的数据库字段，支持序列化/反序列化",
            "steps": [
                "文件: crates/common/src/types.rs",
                "扩展 Price 结构体添加新字段",
                "定义 TieredPrice 结构体 { model, region, tier_start, tier_end, input_price, output_price }",
                "定义 FullPricing 结构体用于 JSON blob",
                "添加 #[derive(Debug, Clone, Serialize, Deserialize)]",
                "实现 Default trait 保持向后兼容"
            ],
            "verification": [
                "cargo check 编译通过",
                "单元测试: JSON 序列化/反序列化正确"
            ],
            "constraints": [
                "使用 Option<T> 包装新字段，默认 None",
                "FullPricing 使用 serde_json::Value 类型"
            ]
        },
        {
            "id": "1.4",
            "passes": true,
            "category": "phase1-database-models",
            "priority": "P2",
            "description": "实现高级定价 CRUD 操作",
            "context": "数据库操作层需要支持读写高级定价字段",
            "steps": [
                "文件: crates/database-models/src/lib.rs",
                "扩展 PriceModel::upsert 支持新字段",
                "实现 TieredPriceModel::get_tiers(model, region) -> Vec<TieredPrice>",
                "实现 TieredPriceModel::upsert_tier",
                "实现 TieredPriceModel::delete_tiers(model, region)",
                "添加批量操作方法"
            ],
            "verification": [
                "cargo test 单元测试通过",
                "能正确读写阶梯定价数据"
            ]
        },
        {
            "id": "2.1",
            "passes": true,
            "category": "phase2-price-sync",
            "priority": "P2",
            "description": "扩展 LiteLLM 价格同步支持缓存定价",
            "context": "LiteLLM JSON 包含 cache_read_input_token_cost 等字段，当前同步忽略这些字段。Prompt Caching 可节省 90% 成本，计费必须准确",
            "steps": [
                "文件: crates/router/src/price_sync.rs",
                "扩展 LiteLLMPrice 结构体添加缓存定价字段",
                "字段: cache_read_input_token_cost: Option<f64>",
                "字段: cache_creation_input_token_cost: Option<f64>",
                "修改 sync_from_litellm 方法写入新字段",
                "添加日志记录同步的缓存定价数量"
            ],
            "verification": [
                "cargo test 单元测试通过",
                "集成测试: 能同步 Claude 3.5 Sonnet 缓存定价数据",
                "验证: cache_read_price = $0.30/1M (原价 $3.00/1M 的 10%)"
            ],
            "references": [
                "LiteLLM JSON 示例: input_cost_per_token: 3e-06, cache_read_input_token_cost: 3e-07"
            ]
        },
        {
            "id": "2.2",
            "passes": true,
            "category": "phase2-price-sync",
            "priority": "P2",
            "description": "扩展 LiteLLM 价格同步支持批量定价",
            "context": "Batch API 可节省 50% 成本，需要同步批量价格字段",
            "steps": [
                "文件: crates/router/src/price_sync.rs",
                "扩展 LiteLLMPrice 结构体",
                "字段: input_cost_per_token_batches: Option<f64>",
                "字段: output_cost_per_token_batches: Option<f64>",
                "修改 sync_from_litellm 写入 batch_*_price 字段"
            ],
            "verification": [
                "cargo test 通过",
                "能同步批量定价数据"
            ]
        },
        {
            "id": "2.3",
            "passes": true,
            "category": "phase2-price-sync",
            "priority": "P3",
            "description": "扩展 LiteLLM 价格同步支持优先级和音频定价",
            "context": "高优先级请求加价 70%，音频 token 价格是文本 7 倍，需要支持精确计费",
            "steps": [
                "文件: crates/router/src/price_sync.rs",
                "扩展 LiteLLMPrice 结构体",
                "字段: input_cost_per_token_priority: Option<f64>",
                "字段: output_cost_per_token_priority: Option<f64>",
                "字段: input_cost_per_audio_token: Option<f64>",
                "字段: search_context_cost_per_query: Option<f64>",
                "修改 sync_from_litellm 写入新字段"
            ],
            "verification": [
                "cargo test 通过",
                "能同步音频和优先级定价"
            ]
        },
        {
            "id": "2.4",
            "passes": true,
            "category": "phase2-price-sync",
            "priority": "P2",
            "description": "实现阶梯定价手动配置接口",
            "context": "LiteLLM 不包含阶梯定价数据，需要手动配置或从其他源导入",
            "steps": [
                "文件: crates/router/src/price_sync.rs",
                "实现 TieredPricingImporter::import_from_json",
                "支持从 JSON 文件导入阶梯定价",
                "实现数据验证: tier_start < tier_end, price > 0",
                "添加 CLI 命令: burncloud pricing import-tiered <file.json>"
            ],
            "verification": [
                "cargo test 通过",
                "能导入 Qwen 阶梯定价示例数据",
                "验证: 导入后数据库包含 6 条 Qwen 记录 (3 阶梯 × 2 区域)"
            ]
        },
        {
            "id": "3.1",
            "passes": true,
            "category": "phase3-billing-logic",
            "priority": "P2",
            "description": "实现阶梯计费核心算法",
            "context": {
                "why": "当前计费公式 tokens * price 无法处理阶梯定价",
                "algorithm": "分段累计：按 tier_start/tier_end 分段计算后求和",
                "example": "150K tokens 按 Qwen 海外版 = 32K×$1.2 + 96K×$2.4 + 22K×$3.0 = $0.3348"
            },
            "steps": [
                {
                    "action": "创建计费模块",
                    "file": "crates/router/src/billing.rs",
                    "type": "新建"
                },
                {
                    "action": "定义阶梯计费函数",
                    "signature": "pub fn calculate_tiered_cost(tokens: u64, tiers: &[TieredPrice], region: Option<&str>) -> Result<f64, BillingError>",
                    "returns": "计算结果（美元），保留 6 位小数"
                },
                {
                    "action": "实现分段累计算法",
                    "pseudocode": "for tier in sorted_tiers:\n  tier_tokens = min(remaining, tier.tier_end - tier.tier_start)\n  cost += tier_tokens * tier.price\n  remaining -= tier_tokens\n  if remaining == 0: break"
                },
                {
                    "action": "处理边界情况",
                    "cases": [
                        "tokens 为 0: 返回 0.0",
                        "tiers 为空: 返回 BillingError::NoTiers",
                        "tokens 超出最后阶梯: 按最后阶梯价格继续计算",
                        "region 不匹配: 使用 region=NULL 的通用阶梯"
                    ]
                },
                {
                    "action": "添加单元测试",
                    "file": "crates/router/src/billing.rs",
                    "tests": [
                        "test_single_tier_equals_simple",
                        "test_multi_tier_accumulation",
                        "test_exceed_last_tier",
                        "test_exact_tier_boundary",
                        "test_zero_tokens",
                        "test_empty_tiers",
                        "test_region_selection"
                    ]
                }
            ],
            "acceptance_criteria": [
                "GIVEN Qwen 海外版阶梯定价配置",
                "WHEN 计算 150K tokens 费用",
                "THEN 结果为 $0.3348",
                "AND 计算时间 < 1μs"
            ],
            "error_handling": [
                "tiers 为空: 返回 Err(BillingError::NoTiers)，调用方回退简单计费",
                "tier_end < tier_start: 返回 Err(BillingError::InvalidTier)",
                "price < 0: 返回 Err(BillingError::InvalidPrice)"
            ],
            "verification": [
                "cargo test billing 通过",
                "测试用例: 150K tokens 按 Qwen 海外版阶梯计算 = $0.3348",
                "测试用例: 20K tokens 按 Qwen 海外版 = 20K × $1.2/1M = $0.024",
                "测试用例: 空阶梯列表返回错误",
                "性能测试: 计算时间 < 1μs"
            ],
            "constraints": [
                "性能要求: 计算时间 < 1μs",
                "精度要求: 使用 f64，保留 6 位小数",
                "禁止 panic: 所有错误通过 Result 返回"
            ],
            "references": [
                "docs/plan.md - 阶梯计费公式示例"
            ]
        },
        {
            "id": "3.2",
            "passes": true,
            "category": "phase3-billing-logic",
            "priority": "P2",
            "description": "实现缓存定价计费逻辑",
            "context": "Prompt Caching 场景，缓存命中 token 价格是标准的 10%，需从响应 usage 中区分 cache_read_tokens",
            "steps": [
                "文件: crates/router/src/billing.rs",
                "定义函数: calculate_cache_cost(standard_tokens, cache_read_tokens, cache_creation_tokens, prices)",
                "解析 Anthropic/OpenAI 响应中的 cache_read_input_tokens 字段",
                "实现: standard_tokens × standard_price + cache_read_tokens × cache_read_price",
                "更新 proxy_handler 调用新计费函数"
            ],
            "verification": [
                "cargo test 通过",
                "测试: 100K standard + 50K cache_read 按 Claude 3.5 计算",
                "预期: 100K×$3 + 50K×$0.30 = $0.315"
            ],
            "references": [
                "Anthropic usage: { prompt_tokens: 100, cache_read_input_tokens: 50 }"
            ]
        },
        {
            "id": "3.3",
            "passes": true,
            "category": "phase3-billing-logic",
            "priority": "P2",
            "description": "实现批量定价计费逻辑",
            "context": "Batch API 请求需要使用批量价格字段计费",
            "steps": [
                "文件: crates/router/src/billing.rs",
                "检测批量请求: 请求体包含 batch 相关字段或 header",
                "使用 batch_input_price/batch_output_price 计费",
                "批量价格缺失时回退到标准价格 + 警告日志"
            ],
            "verification": [
                "cargo test 通过",
                "批量请求使用批量价格计费"
            ]
        },
        {
            "id": "3.4",
            "passes": true,
            "category": "phase3-billing-logic",
            "priority": "P3",
            "description": "实现优先级和音频定价计费逻辑",
            "context": "高优先级请求和音频 token 有不同定价",
            "steps": [
                "文件: crates/router/src/billing.rs",
                "实现音频 token 计费: audio_tokens × audio_input_price",
                "实现优先级计费检测: 请求 metadata.priority",
                "组合计费: 分别计算各类型 token 费用后求和"
            ],
            "verification": [
                "cargo test 通过",
                "音频 token 正确计费"
            ]
        },
        {
            "id": "4.1",
            "passes": true,
            "category": "phase4-integration",
            "priority": "P2",
            "description": "集成高级计费到 proxy_handler",
            "context": "当前 proxy_handler 使用简单计费公式，需要集成新的计费逻辑",
            "steps": [
                "文件: crates/router/src/lib.rs (proxy_handler)",
                "检查模型是否有阶梯定价配置",
                "检查响应是否包含缓存 token 信息",
                "根据情况选择计费方式: 阶梯/缓存/批量/标准",
                "更新 router_logs 记录计费详情",
                "保持向后兼容: 无高级定价时使用原逻辑"
            ],
            "verification": [
                "cargo test 通过",
                "集成测试: Qwen 模型正确使用阶梯计费",
                "集成测试: Claude 模型正确使用缓存计费",
                "回归测试: 普通模型计费不受影响"
            ],
            "constraints": [
                "计费失败不应阻塞请求，使用标准价格 + 错误日志",
                "性能: 计费逻辑增加 < 5ms 延迟"
            ]
        },
        {
            "id": "4.2",
            "passes": true,
            "category": "phase4-integration",
            "priority": "P3",
            "description": "实现区域定价支持",
            "context": "Qwen 国内版价格是海外版的 30%，需要根据渠道配置选择区域定价",
            "steps": [
                "文件: crates/database/src/schema.rs",
                "扩展 channels 表: ADD COLUMN pricing_region TEXT DEFAULT 'international'",
                "文件: crates/router/src/billing.rs",
                "查询阶梯定价时传入 region 参数",
                "支持 'cn', 'international', NULL (通用)"
            ],
            "verification": [
                "cargo test 通过",
                "国内版渠道使用 cn 定价"
            ]
        },
        {
            "id": "4.3",
            "passes": true,
            "category": "phase4-integration",
            "priority": "P3",
            "description": "实现自动检测阶梯计费模型",
            "context": "自动识别模型是否需要阶梯计费，无需手动配置",
            "steps": [
                "文件: crates/router/src/billing.rs",
                "实现函数: needs_tiered_pricing(model) -> bool",
                "检测逻辑: 查询 tiered_pricing 表是否有该模型记录",
                "添加 CLI 命令: burncloud pricing check-tiered <model>",
                "在 admin UI 显示模型计费类型"
            ],
            "verification": [
                "cargo test 通过",
                "Qwen 模型返回 true",
                "GPT-4 返回 false"
            ]
        },
        {
            "id": "5.1",
            "passes": true,
            "category": "phase5-cli",
            "priority": "P2",
            "description": "实现阶梯定价 CLI 管理命令",
            "context": "管理员需要 CLI 工具管理阶梯定价配置",
            "steps": [
                "文件: crates/cli/src/pricing.rs (新建)",
                "命令: burncloud pricing list-tiers --model qwen3-max",
                "命令: burncloud pricing add-tier --model qwen3-max --region cn --tier-start 0 --tier-end 32000 --input-price 0.359 --output-price 1.434",
                "命令: burncloud pricing import-tiered <file.json>",
                "命令: burncloud pricing export-tiered --model qwen3-max > tiers.json",
                "文件: crates/cli/src/commands.rs - 注册 pricing 子命令"
            ],
            "verification": [
                "cargo build 编译通过",
                "手动测试: 添加/查询/导出阶梯定价"
            ]
        },
        {
            "id": "5.2",
            "passes": null,
            "category": "phase5-cli",
            "priority": "P3",
            "description": "实现高级定价同步状态 CLI",
            "context": "查看价格同步是否包含高级定价字段",
            "steps": [
                "文件: crates/cli/src/pricing.rs",
                "命令: burncloud pricing show <model> --verbose",
                "显示: 标准价格、缓存价格、批量价格、阶梯定价配置",
                "命令: burncloud pricing sync-status",
                "显示: 上次同步时间、同步的高级字段数量"
            ],
            "verification": [
                "cargo build 通过",
                "手动测试命令输出正确"
            ]
        },
        {
            "id": "6.1",
            "passes": true,
            "category": "phase6-tests",
            "priority": "P2",
            "description": "编写阶梯计费单元测试",
            "context": "确保阶梯计费算法正确，覆盖各种边界情况",
            "steps": [
                "文件: crates/router/src/billing.rs (内联测试)",
                "测试: 单阶梯等于简单计费",
                "测试: 多阶梯正确分段累加",
                "测试: tokens 超出最后阶梯",
                "测试: tokens 刚好等于阶梯边界",
                "测试: tokens 为 0",
                "测试: 阶梯列表为空回退简单计费",
                "测试: 区域定价正确选择"
            ],
            "verification": [
                "cargo test billing 通过",
                "测试覆盖率 > 90%"
            ]
        },
        {
            "id": "6.2",
            "passes": true,
            "category": "phase6-tests",
            "priority": "P2",
            "description": "编写缓存计费单元测试",
            "context": "确保 Prompt Caching 计费准确",
            "steps": [
                "文件: crates/router/src/billing.rs (内联测试)",
                "测试: 纯标准 token 计费",
                "测试: 标准 + 缓存命中混合计费",
                "测试: 缓存创建计费",
                "测试: 价格缺失回退标准价格"
            ],
            "verification": [
                "cargo test billing 通过",
                "预期: 100K standard + 50K cache = $0.315 (Claude 3.5)"
            ]
        },
        {
            "id": "6.3",
            "passes": true,
            "category": "phase6-tests",
            "priority": "P2",
            "description": "编写价格同步集成测试",
            "context": "确保 LiteLLM 高级字段正确同步到数据库",
            "steps": [
                "文件: crates/tests/pricing_test.rs (新建)",
                "模拟 LiteLLM JSON 响应包含高级字段",
                "测试: 同步后 prices 表包含缓存价格",
                "测试: 同步后 prices 表包含批量价格",
                "测试: 字段值为 NULL 时正确处理"
            ],
            "verification": [
                "cargo test --test pricing_test 通过"
            ]
        },
        {
            "id": "7.1",
            "passes": true,
            "category": "phase7-docs",
            "priority": "P3",
            "description": "更新 CLAUDE.md 添加高级定价说明",
            "context": "让开发者了解新的高级定价功能",
            "steps": [
                "文件: CLAUDE.md",
                "添加 billing.rs 模块说明",
                "添加阶梯定价使用示例",
                "添加缓存定价使用示例",
                "更新价格同步说明"
            ],
            "verification": [
                "文档审核通过"
            ]
        },
        {
            "id": "7.2",
            "passes": true,
            "category": "phase7-docs",
            "priority": "P3",
            "description": "创建阶梯定价配置示例文件",
            "context": "提供 Qwen 等模型的阶梯定价配置示例",
            "steps": [
                "文件: config/tiered_pricing_example.json",
                "包含 Qwen 国内版和海外版阶梯定价",
                "包含 DeepSeek 阶梯定价 (如有)",
                "添加 JSON schema 定义"
            ],
            "verification": [
                "JSON 格式正确",
                "可通过 CLI import 命令导入"
            ]
        },
        {
            "id": "8.1",
            "passes": null,
            "category": "phase8-multi-currency",
            "priority": "P2",
            "description": "扩展数据库支持多货币字段",
            "context": {
                "why": "当前价格系统假设单一货币（USD），无法处理人民币定价的中国模型",
                "impact": "价格表、日志表、用户余额都需要支持多货币",
                "risk": "汇率波动可能导致展示价格不稳定"
            },
            "steps": [
                {
                    "action": "扩展 prices 表增加货币字段",
                    "file": "crates/database/src/schema.rs",
                    "fields": [
                        "currency TEXT DEFAULT 'USD'",
                        "original_currency TEXT",
                        "original_input_price REAL",
                        "original_output_price REAL"
                    ]
                },
                {
                    "action": "创建汇率表 exchange_rates",
                    "file": "crates/database/src/schema.rs",
                    "fields": [
                        "id INTEGER PRIMARY KEY",
                        "from_currency TEXT NOT NULL",
                        "to_currency TEXT NOT NULL",
                        "rate REAL NOT NULL",
                        "updated_at DATETIME DEFAULT CURRENT_TIMESTAMP"
                    ],
                    "constraint": "UNIQUE(from_currency, to_currency)"
                },
                {
                    "action": "创建迁移脚本",
                    "file": "migrations/YYYYMMDDHHMMSS_add_multi_currency/up.sql"
                }
            ],
            "acceptance_criteria": [
                "GIVEN prices 表有 USD 价格数据",
                "WHEN 执行迁移",
                "THEN 新字段默认为 USD，现有数据完整",
                "AND cargo build 编译通过"
            ],
            "verification": [
                "cargo build 编译通过",
                "数据库迁移成功",
                "能插入 CNY 价格记录",
                "能插入汇率数据"
            ],
            "constraints": [
                "默认货币为 USD 保持向后兼容",
                "支持 USD, CNY, EUR 三种货币",
                "汇率精度保留 6 位小数"
            ],
            "references": [
                "docs/plan.md 第十四章 - 多货币支持"
            ]
        },
        {
            "id": "8.2",
            "passes": null,
            "category": "phase8-multi-currency",
            "priority": "P2",
            "description": "定义多货币数据结构和枚举",
            "context": "需要 Rust 枚举和结构体支持多货币类型",
            "steps": [
                "文件: crates/common/src/types.rs",
                "定义 Currency 枚举: USD, CNY, EUR",
                "添加 #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]",
                "实现 Display trait 用于格式化输出",
                "定义 MultiCurrencyPrice 结构体",
                "定义 ExchangeRate 结构体",
                "实现货币符号映射: USD→$, CNY→¥, EUR→€"
            ],
            "verification": [
                "cargo check 编译通过",
                "单元测试: Currency::from_str 正确解析",
                "单元测试: 货币符号正确显示"
            ],
            "constraints": [
                "Currency 使用 enum 而非 String 提高性能",
                "序列化时使用小写字符串: \"usd\", \"cny\", \"eur\""
            ]
        },
        {
            "id": "8.3",
            "passes": null,
            "category": "phase8-multi-currency",
            "priority": "P2",
            "description": "实现汇率服务 ExchangeRateService",
            "context": "需要服务层管理汇率查询和转换",
            "steps": [
                {
                    "action": "创建汇率服务模块",
                    "file": "crates/router/src/exchange_rate.rs (新建)"
                },
                {
                    "action": "实现 ExchangeRateService 结构体",
                    "fields": [
                        "db: Database",
                        "rates: DashMap<(Currency, Currency), f64>",
                        "last_updated: DateTime<Utc>"
                    ]
                },
                {
                    "action": "实现 convert 方法",
                    "signature": "pub fn convert(&self, amount: f64, from: Currency, to: Currency) -> f64"
                },
                {
                    "action": "实现 refresh_rates 方法",
                    "signature": "pub async fn refresh_rates(&mut self) -> Result<()>",
                    "detail": "从数据库加载汇率，支持手动更新"
                },
                {
                    "action": "实现 get_rate 方法",
                    "signature": "pub fn get_rate(&self, from: Currency, to: Currency) -> Option<f64>"
                }
            ],
            "verification": [
                "cargo test 通过",
                "测试: USD→CNY 转换正确",
                "测试: 相同货币返回原值",
                "测试: 缺失汇率返回 None"
            ],
            "constraints": [
                "汇率缓存使用 DashMap 避免锁竞争",
                "每日更新一次即可，无需实时"
            ]
        },
        {
            "id": "8.4",
            "passes": null,
            "category": "phase8-multi-currency",
            "priority": "P3",
            "description": "实现汇率自动更新任务",
            "context": "每日自动更新汇率数据，支持接入外部 API",
            "steps": [
                "文件: crates/router/src/exchange_rate.rs",
                "实现 start_exchange_rate_sync_task 后台任务",
                "每小时检查是否需要更新",
                "支持从外部 API 获取汇率（可配置）",
                "失败时保留旧汇率并记录警告",
                "在 Router 启动时启动同步任务"
            ],
            "verification": [
                "cargo test 通过",
                "集成测试: 能从 API 获取汇率",
                "集成测试: 失败时保留旧数据"
            ],
            "constraints": [
                "外部 API 调用有超时限制（5秒）",
                "支持配置关闭自动更新"
            ],
            "references": [
                "常用汇率 API: exchangerate-api.com, fixer.io"
            ]
        },
        {
            "id": "8.5",
            "passes": null,
            "category": "phase8-multi-currency",
            "priority": "P2",
            "description": "集成多货币到计费逻辑",
            "context": "计费时根据渠道货币配置选择对应价格",
            "steps": [
                {
                    "action": "扩展 billing.rs 支持多货币",
                    "file": "crates/router/src/billing.rs"
                },
                {
                    "action": "修改 calculate_cost 函数",
                    "changes": [
                        "增加 currency 参数",
                        "优先查询本地货币价格",
                        "回退到 USD 价格",
                        "返回双货币结果"
                    ]
                },
                {
                    "action": "定义 CostResult 结构体",
                    "fields": [
                        "usd_amount: f64",
                        "local_currency: Currency",
                        "local_amount: Option<f64>",
                        "display: String"
                    ]
                }
            ],
            "verification": [
                "cargo test 通过",
                "测试: CNY 渠道返回人民币价格",
                "测试: 缺失本地价格时回退 USD"
            ],
            "constraints": [
                "内部计算统一使用 USD 保证精度",
                "展示时转换为本地货币"
            ]
        },
        {
            "id": "8.6",
            "passes": null,
            "category": "phase8-multi-currency",
            "priority": "P3",
            "description": "实现用户偏好货币展示",
            "context": "用户可选择以哪种货币查看消费",
            "steps": [
                "文件: crates/database/src/schema.rs",
                "扩展 users 表: ADD COLUMN preferred_currency TEXT DEFAULT 'USD'",
                "文件: crates/router/src/lib.rs",
                "API 响应中包含多货币成本展示",
                "支持 ?currency=CNY 查询参数覆盖用户偏好"
            ],
            "verification": [
                "cargo test 通过",
                "用户偏好 CNY 时展示人民币价格",
                "查询参数可覆盖用户偏好"
            ]
        },
        {
            "id": "8.7",
            "passes": null,
            "category": "phase8-multi-currency",
            "priority": "P3",
            "description": "实现汇率管理 CLI 命令",
            "context": "管理员需要手动管理汇率",
            "steps": [
                "文件: crates/cli/src/currency.rs (新建)",
                "命令: burncloud currency list-rates",
                "命令: burncloud currency set-rate --from CNY --to USD --rate 0.14",
                "命令: burncloud currency refresh (手动触发更新)",
                "文件: crates/cli/src/commands.rs - 注册 currency 子命令"
            ],
            "verification": [
                "cargo build 编译通过",
                "手动测试: 能设置和查询汇率"
            ]
        },
        {
            "id": "8.8",
            "passes": null,
            "category": "phase8-multi-currency",
            "priority": "P3",
            "description": "编写多货币单元测试和集成测试",
            "context": "确保货币转换和计费逻辑正确",
            "steps": [
                "文件: crates/router/src/exchange_rate.rs (内联测试)",
                "测试: 货币转换正确性",
                "测试: 汇率缺失处理",
                "测试: 双货币计费结果",
                "文件: crates/tests/currency_test.rs (新建)",
                "集成测试: 多货币价格查询"
            ],
            "verification": [
                "cargo test 通过",
                "测试覆盖率 > 90%"
            ]
        },
        {
            "id": "9.1",
            "passes": true,
            "category": "phase9-pricing-source",
            "priority": "P1",
            "description": "定义 pricing.json Schema 和数据结构",
            "context": {
                "why": "需要标准化的价格配置格式，支持多货币、阶梯定价、缓存定价等高级特性",
                "impact": "所有价格同步和管理功能的基础",
                "risk": "Schema 设计不合理会导致后续扩展困难"
            },
            "steps": [
                {
                    "action": "创建 pricing.schema.json",
                    "file": "config/schemas/pricing.schema.json",
                    "detail": "JSON Schema 定义，包含所有定价字段验证规则"
                },
                {
                    "action": "定义 Rust 数据结构",
                    "file": "crates/common/src/pricing_config.rs (新建)",
                    "structures": [
                        "PricingConfig { version, updated_at, source, models }",
                        "ModelPricing { pricing, tiered_pricing, cache_pricing, batch_pricing, metadata }",
                        "CurrencyPricing { input_price, output_price, source }",
                        "TieredPrice { tier_start, tier_end, input_price, output_price }",
                        "CachePricing { cache_read_price, cache_creation_price }",
                        "ModelMetadata { context_window, max_output_tokens, supports_vision, ... }"
                    ]
                },
                {
                    "action": "实现 JSON 序列化/反序列化",
                    "derives": [
                        "#[derive(Debug, Clone, Serialize, Deserialize)]"
                    ]
                },
                {
                    "action": "实现配置验证方法",
                    "signature": "pub fn validate(&self) -> Result<Vec<ValidationWarning>>"
                }
            ],
            "acceptance_criteria": [
                "GIVEN 一个有效的 pricing.json 文件",
                "WHEN 调用 PricingConfig::from_json",
                "THEN 成功解析所有字段",
                "AND 验证方法返回空警告列表"
            ],
            "verification": [
                "cargo check 编译通过",
                "单元测试: 解析示例 pricing.json 成功",
                "单元测试: 无效配置返回错误"
            ],
            "constraints": [
                "使用 serde 进行 JSON 处理",
                "所有价格字段使用 Option<f64> 支持部分定价",
                "version 字段格式为 'X.Y'"
            ],
            "references": [
                "docs/plan.md 第十五章 - pricing.json Schema"
            ]
        },
        {
            "id": "9.2",
            "passes": null,
            "category": "phase9-pricing-source",
            "priority": "P1",
            "description": "创建 prices_v2 多货币价格表",
            "context": "原有 prices 表不支持多货币，需要新建 prices_v2 表替代",
            "steps": [
                {
                    "action": "创建 prices_v2 表",
                    "file": "crates/database/src/schema.rs",
                    "fields": [
                        "id INTEGER PRIMARY KEY",
                        "model TEXT NOT NULL",
                        "currency TEXT NOT NULL",
                        "input_price REAL NOT NULL",
                        "output_price REAL NOT NULL",
                        "cache_read_input_price REAL",
                        "cache_creation_input_price REAL",
                        "batch_input_price REAL",
                        "batch_output_price REAL",
                        "priority_input_price REAL",
                        "priority_output_price REAL",
                        "audio_input_price REAL",
                        "source TEXT",
                        "region TEXT",
                        "context_window INTEGER",
                        "max_output_tokens INTEGER",
                        "supports_vision BOOLEAN DEFAULT FALSE",
                        "supports_function_calling BOOLEAN DEFAULT FALSE",
                        "synced_at DATETIME",
                        "created_at DATETIME",
                        "updated_at DATETIME"
                    ],
                    "constraint": "UNIQUE(model, currency, region)"
                },
                {
                    "action": "创建数据库迁移",
                    "file": "migrations/YYYYMMDDHHMMSS_create_prices_v2/up.sql",
                    "steps": [
                        "CREATE TABLE prices_v2 ...",
                        "CREATE INDEX idx_prices_v2_model ON prices_v2(model)",
                        "CREATE INDEX idx_prices_v2_model_currency ON prices_v2(model, currency)",
                        "INSERT INTO prices_v2 SELECT ... FROM prices (迁移现有数据)"
                    ]
                },
                {
                    "action": "更新 tiered_pricing 表",
                    "changes": [
                        "ALTER TABLE tiered_pricing ADD COLUMN currency TEXT DEFAULT 'USD'",
                        "更新 UNIQUE 约束为 (model, region, currency, tier_start)"
                    ]
                }
            ],
            "verification": [
                "cargo build 编译通过",
                "数据库迁移成功",
                "现有 USD 价格数据正确迁移"
            ],
            "rollback": {
                "postgresql": "DROP TABLE prices_v2; DROP INDEX idx_prices_v2_model;",
                "sqlite": "DROP TABLE prices_v2;"
            }
        },
        {
            "id": "9.3",
            "passes": null,
            "category": "phase9-pricing-source",
            "priority": "P2",
            "description": "实现本地配置文件加载服务",
            "context": "最高优先级的价格数据源，支持手动维护价格配置",
            "steps": [
                {
                    "action": "创建价格配置加载器",
                    "file": "crates/router/src/pricing_loader.rs (新建)",
                    "functions": [
                        "load_local_override() -> Option<PricingConfig>",
                        "load_local_config() -> Option<PricingConfig>",
                        "validate_config(config: &PricingConfig) -> Result<()>"
                    ]
                },
                {
                    "action": "实现配置文件监视",
                    "detail": "使用 notify crate 监听文件变更，自动重新加载"
                },
                {
                    "action": "添加配置路径配置",
                    "fields": [
                        "local_config_path: PathBuf = 'config/pricing.json'",
                        "override_config_path: PathBuf = 'config/pricing.override.json'"
                    ]
                }
            ],
            "verification": [
                "cargo test 通过",
                "测试: 加载有效的 pricing.json",
                "测试: 缺失文件返回 None",
                "测试: 无效 JSON 返回错误"
            ]
        },
        {
            "id": "9.4",
            "passes": null,
            "category": "phase9-pricing-source",
            "priority": "P2",
            "description": "实现价格同步服务 V2",
            "context": "替代原有 PriceSyncService，支持多数据源和多货币",
            "steps": [
                {
                    "action": "创建 PriceSyncServiceV2",
                    "file": "crates/router/src/price_sync.rs (重构)",
                    "fields": [
                        "db: Database",
                        "http_client: reqwest::Client",
                        "local_config_path: PathBuf",
                        "override_config_path: PathBuf",
                        "community_repo_url: String",
                        "litellm_url: String",
                        "last_community_sync: DateTime<Utc>",
                        "last_litellm_sync: DateTime<Utc>"
                    ]
                },
                {
                    "action": "实现 sync_all 方法",
                    "logic": [
                        "1. 加载本地覆盖配置（最高优先级）",
                        "2. 加载本地主配置",
                        "3. 同步社区价格库（每日）",
                        "4. 同步 LiteLLM（仅 USD 回退）"
                    ]
                },
                {
                    "action": "实现 apply_prices 方法",
                    "signature": "async fn apply_prices(&self, config: &PricingConfig, source: &str) -> Result<SyncResult>"
                },
                {
                    "action": "实现 upsert_price 方法",
                    "detail": "使用 INSERT OR REPLACE 实现幂等更新"
                }
            ],
            "verification": [
                "cargo test 通过",
                "测试: 多数据源优先级正确",
                "测试: 同步结果统计准确"
            ]
        },
        {
            "id": "9.5",
            "passes": null,
            "category": "phase9-pricing-source",
            "priority": "P2",
            "description": "实现价格导入导出 CLI 命令",
            "context": "管理员需要 CLI 工具管理价格配置",
            "steps": [
                {
                    "action": "扩展 pricing CLI 模块",
                    "file": "crates/cli/src/pricing.rs"
                },
                {
                    "action": "实现 import 命令",
                    "usage": "burncloud pricing import <file.json> [--override]",
                    "behavior": "验证 JSON 格式，写入数据库"
                },
                {
                    "action": "实现 export 命令",
                    "usage": "burncloud pricing export [--format json|csv] > pricing.json",
                    "behavior": "从数据库导出所有价格为标准格式"
                },
                {
                    "action": "实现 validate 命令",
                    "usage": "burncloud pricing validate <file.json>",
                    "behavior": "验证 JSON 格式和 schema 合规性"
                }
            ],
            "verification": [
                "cargo build 编译通过",
                "手动测试: import/export/validate 命令正常工作"
            ]
        },
        {
            "id": "9.6",
            "passes": null,
            "category": "phase9-pricing-source",
            "priority": "P2",
            "description": "实现价格设置和查询 CLI 命令",
            "context": "管理员需要手动设置和查询特定模型价格",
            "steps": [
                {
                    "action": "实现 set 命令",
                    "usage": "burncloud pricing set <model> --currency CNY --input-price 0.002 --output-price 0.006 --region cn"
                },
                {
                    "action": "实现 show 命令",
                    "usage": "burncloud pricing show <model> [--currency CNY]",
                    "output": "显示所有货币价格、阶梯定价、缓存定价等"
                },
                {
                    "action": "实现 list 命令",
                    "usage": "burncloud pricing list [--currency CNY] [--provider openai]",
                    "output": "表格形式显示模型列表和价格"
                },
                {
                    "action": "实现 set-tier 命令",
                    "usage": "burncloud pricing set-tier <model> --currency CNY --tier-start 0 --tier-end 32000 --input-price 0.359"
                },
                {
                    "action": "实现 list-tiers 命令",
                    "usage": "burncloud pricing list-tiers <model> [--currency CNY]"
                }
            ],
            "verification": [
                "cargo build 编译通过",
                "手动测试所有命令"
            ]
        },
        {
            "id": "9.7",
            "passes": null,
            "category": "phase9-pricing-source",
            "priority": "P3",
            "description": "实现社区价格库同步",
            "context": "从 GitHub 社区仓库同步价格数据，作为次优先级数据源",
            "steps": [
                {
                    "action": "实现 fetch_community_prices 方法",
                    "file": "crates/router/src/price_sync.rs",
                    "url": "https://raw.githubusercontent.com/burncloud/pricing-data/main/pricing/latest.json"
                },
                {
                    "action": "实现增量同步",
                    "logic": [
                        "检查 If-Modified-Since 头",
                        "仅同步变更的模型",
                        "记录同步时间戳"
                    ]
                },
                {
                    "action": "添加同步配置",
                    "fields": [
                        "community_sync_enabled: bool = true",
                        "community_sync_interval: Duration = 24h"
                    ]
                }
            ],
            "verification": [
                "cargo test 通过",
                "集成测试: 能获取社区价格",
                "集成测试: 增量同步正确"
            ]
        },
        {
            "id": "9.8",
            "passes": true,
            "category": "phase9-pricing-source",
            "priority": "P3",
            "description": "创建示例 pricing.json 文件",
            "context": "提供开箱即用的价格配置示例",
            "steps": [
                {
                    "action": "创建示例配置",
                    "file": "config/pricing.example.json",
                    "models": [
                        "gpt-4-turbo (USD + CNY)",
                        "qwen-max (USD + CNY + 阶梯定价)",
                        "claude-3-5-sonnet (USD + 缓存定价)",
                        "deepseek-chat (USD + CNY)"
                    ]
                },
                {
                    "action": "创建 README",
                    "file": "config/README.md",
                    "content": "配置说明、字段解释、使用示例"
                }
            ],
            "verification": [
                "JSON 格式正确",
                "通过 schema 验证",
                "可通过 CLI import 导入"
            ]
        },
        {
            "id": "9.9",
            "passes": null,
            "category": "phase9-pricing-source",
            "priority": "P3",
            "description": "编写价格同步集成测试",
            "context": "确保多数据源同步逻辑正确",
            "steps": [
                "文件: crates/tests/pricing_sync_test.rs (新建)",
                "测试: 本地配置优先级最高",
                "测试: 社区价格正确覆盖 LiteLLM",
                "测试: 多货币价格正确存储",
                "测试: 阶梯定价正确同步",
                "测试: 同步失败时保留旧数据"
            ],
            "verification": [
                "cargo test --test pricing_sync_test 通过",
                "测试覆盖率 > 85%"
            ]
        }
    ],
    "dependencies": {
        "1.2": [
            "1.1"
        ],
        "1.3": [
            "1.1"
        ],
        "1.4": [
            "1.2",
            "1.3"
        ],
        "2.1": [
            "1.1",
            "1.3"
        ],
        "2.2": [
            "1.1",
            "1.3"
        ],
        "2.3": [
            "1.1",
            "1.3"
        ],
        "2.4": [
            "1.2",
            "1.4"
        ],
        "3.1": [
            "1.4"
        ],
        "3.2": [
            "1.1",
            "1.3"
        ],
        "3.3": [
            "1.1",
            "1.3"
        ],
        "3.4": [
            "1.1",
            "1.3"
        ],
        "4.1": [
            "3.1",
            "3.2",
            "3.3"
        ],
        "4.2": [
            "1.2",
            "3.1"
        ],
        "4.3": [
            "1.4",
            "3.1"
        ],
        "5.1": [
            "1.4"
        ],
        "5.2": [
            "2.1",
            "2.2"
        ],
        "6.1": [
            "3.1"
        ],
        "6.2": [
            "3.2"
        ],
        "6.3": [
            "2.1"
        ],
        "7.1": [
            "4.1"
        ],
        "7.2": [
            "2.4"
        ],
        "8.2": [
            "8.1"
        ],
        "8.3": [
            "8.1",
            "8.2"
        ],
        "8.4": [
            "8.3"
        ],
        "8.5": [
            "8.1",
            "8.2",
            "8.3"
        ],
        "8.6": [
            "8.5"
        ],
        "8.7": [
            "8.3"
        ],
        "8.8": [
            "8.3",
            "8.5"
        ],
        "9.2": [
            "9.1"
        ],
        "9.3": [
            "9.1"
        ],
        "9.4": [
            "9.1",
            "9.2",
            "9.3"
        ],
        "9.5": [
            "9.4"
        ],
        "9.6": [
            "9.4"
        ],
        "9.7": [
            "9.4"
        ],
        "9.8": [
            "9.1"
        ],
        "9.9": [
            "9.4"
        ]
    },
    "test_data": {
        "qwen_tiered_pricing": {
            "model": "qwen3-max",
            "tiers": [
                {
                    "region": "cn",
                    "tier_start": 0,
                    "tier_end": 32000,
                    "input_price": 0.359,
                    "output_price": 1.434
                },
                {
                    "region": "cn",
                    "tier_start": 32000,
                    "tier_end": 128000,
                    "input_price": 0.574,
                    "output_price": 2.294
                },
                {
                    "region": "cn",
                    "tier_start": 128000,
                    "tier_end": 252000,
                    "input_price": 1.004,
                    "output_price": 4.014
                },
                {
                    "region": "international",
                    "tier_start": 0,
                    "tier_end": 32000,
                    "input_price": 1.2,
                    "output_price": 6.0
                },
                {
                    "region": "international",
                    "tier_start": 32000,
                    "tier_end": 128000,
                    "input_price": 2.4,
                    "output_price": 12.0
                },
                {
                    "region": "international",
                    "tier_start": 128000,
                    "tier_end": 252000,
                    "input_price": 3.0,
                    "output_price": 15.0
                }
            ]
        },
        "claude_cache_pricing": {
            "model": "claude-3-5-sonnet-20241022",
            "input_price": 3.0,
            "cache_read_price": 0.3,
            "cache_creation_price": 3.75,
            "output_price": 15.0
        },
        "multi_currency_prices": {
            "qwen-max-cn": {
                "model": "qwen-max",
                "currency": "CNY",
                "input_price": 0.002,
                "output_price": 0.006
            },
            "qwen-max-intl": {
                "model": "qwen-max",
                "currency": "USD",
                "input_price": 0.0012,
                "output_price": 0.006
            },
            "gpt-4": {
                "model": "gpt-4-turbo",
                "currency": "USD",
                "input_price": 0.01,
                "output_price": 0.03
            }
        },
        "exchange_rates": {
            "CNY_to_USD": 0.14,
            "USD_to_CNY": 7.2,
            "EUR_to_USD": 1.08,
            "USD_to_EUR": 0.93
        },
        "pricing_json_example": {
            "version": "1.0",
            "updated_at": "2024-01-15T10:00:00Z",
            "source": "local",
            "models": {
                "gpt-4-turbo": {
                    "pricing": {
                        "USD": {
                            "input_price": 10.0,
                            "output_price": 30.0,
                            "source": "openai"
                        },
                        "CNY": {
                            "input_price": 72.0,
                            "output_price": 216.0,
                            "source": "converted"
                        }
                    },
                    "cache_pricing": {
                        "USD": {
                            "cache_read_input_price": 1.0,
                            "cache_creation_input_price": 1.25
                        }
                    },
                    "metadata": {
                        "context_window": 128000,
                        "max_output_tokens": 4096,
                        "supports_vision": true,
                        "supports_function_calling": true,
                        "provider": "openai"
                    }
                },
                "qwen-max": {
                    "pricing": {
                        "USD": {
                            "input_price": 1.2,
                            "output_price": 6.0,
                            "source": "international"
                        },
                        "CNY": {
                            "input_price": 0.359,
                            "output_price": 1.434,
                            "source": "cn"
                        }
                    },
                    "tiered_pricing": {
                        "USD": [
                            {
                                "tier_start": 0,
                                "tier_end": 32000,
                                "input_price": 1.2,
                                "output_price": 6.0
                            },
                            {
                                "tier_start": 32000,
                                "tier_end": 128000,
                                "input_price": 2.4,
                                "output_price": 12.0
                            }
                        ],
                        "CNY": [
                            {
                                "tier_start": 0,
                                "tier_end": 32000,
                                "input_price": 0.359,
                                "output_price": 1.434
                            }
                        ]
                    }
                }
            }
        }
    },
    "api_contracts": {
        "TieredPriceModel": {
            "get_tiers": {
                "input": {
                    "model": "String",
                    "region": "Option<String>"
                },
                "output": "Vec<TieredPrice>",
                "errors": [
                    "DatabaseError"
                ]
            },
            "upsert_tier": {
                "input": {
                    "tier": "TieredPrice"
                },
                "output": "Result<(), DatabaseError>",
                "idempotent": true
            }
        },
        "BillingService": {
            "calculate_cost": {
                "input": {
                    "model": "String",
                    "region": "Option<String>",
                    "usage": "TokenUsage"
                },
                "output": "Result<Cost, BillingError>",
                "hot_path": true
            }
        }
    },
    "summary": {
        "critical_path": [
            "1.1",
            "1.2",
            "1.4",
            "3.1",
            "4.1",
            "8.1",
            "8.3",
            "8.5",
            "9.1",
            "9.2",
            "9.4"
        ],
        "parallelizable": [
            [
                "1.2",
                "1.3"
            ],
            [
                "2.1",
                "2.2",
                "2.3"
            ],
            [
                "3.1",
                "3.2",
                "3.3",
                "3.4"
            ],
            [
                "4.2",
                "4.3",
                "5.1",
                "5.2"
            ],
            [
                "6.1",
                "6.2",
                "6.3"
            ],
            [
                "8.1",
                "8.2"
            ],
            [
                "8.4",
                "8.5",
                "8.6",
                "8.7"
            ],
            [
                "9.1",
                "9.2"
            ],
            [
                "9.3",
                "9.4"
            ],
            [
                "9.5",
                "9.6"
            ],
            [
                "9.7",
                "9.8",
                "9.9"
            ]
        ],
        "risk_areas": [
            {
                "area": "数据库迁移",
                "risk": "迁移失败导致服务无法启动",
                "mitigation": "先在测试环境验证，准备回滚脚本"
            },
            {
                "area": "计费精度",
                "risk": "浮点精度导致计费误差",
                "mitigation": "使用 Decimal 类型或定点数，添加精度测试"
            },
            {
                "area": "性能影响",
                "risk": "阶梯计费增加请求延迟",
                "mitigation": "使用缓存，热路径避免数据库查询"
            },
            {
                "area": "汇率波动",
                "risk": "汇率变化导致展示价格不稳定",
                "mitigation": "每日更新一次，保留历史汇率记录"
            },
            {
                "area": "价格数据源迁移",
                "risk": "从 LiteLLM 迁移到自定义数据源可能遗漏模型",
                "mitigation": "保留 LiteLLM 作为回退数据源，建立完整测试覆盖"
            }
        ],
        "definition_of_done": [
            "所有 P1 和 P2 任务完成并通过验收",
            "所有单元测试和集成测试通过",
            "代码审查通过",
            "文档更新完成",
            "手动测试验证关键场景"
        ]
    }
}