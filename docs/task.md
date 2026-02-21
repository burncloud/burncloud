{
    "meta": {
        "title": "价格系统 u64 精度迁移 - 实施任务清单",
        "version": "1.0",
        "source": "docs/plan.md 第十七章",
        "priority_order": [
            "P1",
            "P2"
        ],
        "estimated_phases": 6,
        "total_tasks": 14,
        "estimated_effort": "3-5 days",
        "task_status_field": {
            "passes": "null=未开始, true=已完成, false=失败需重试"
        }
    },
    "context": {
        "problem": "当前价格系统使用 f64 类型存储价格，存在浮点精度问题：累加计算时可能产生微小误差、直接比较浮点数不可靠、财务计算应使用精确数值",
        "goal": "将所有价格字段从 f64 迁移到 u64，使用纳美元 (Nanodollars) 作为内部存储单位，实现 9 位小数精度",
        "constraints": [
            "保持 API 向后兼容，JSON 输出仍显示浮点格式",
            "CLI 输入接受 f64，内部转换为 u64",
            "数据库迁移需支持 SQLite 和 PostgreSQL",
            "总价计算使用 u128 中间值避免溢出"
        ],
        "negative_constraints": [
            "不要在计费逻辑中使用 unwrap() 或 expect()",
            "不要在热路径中同步查询数据库",
            "不要直接删除旧价格数据",
            "不要在迁移时跳过数据验证",
            "不要在溢出时静默截断"
        ],
        "security": {
            "input_validation": "所有价格输入必须 >= 0",
            "overflow_protection": "使用 u128 中间值计算总价",
            "sql_injection": "使用参数化查询"
        },
        "metrics": {
            "precision": "9 位小数精度 (纳美元)",
            "billing_accuracy": "计费误差 < 10^-9",
            "overflow_threshold": "单次请求最大 10B tokens × $1000/1M"
        }
    },
    "execution_order": [
        ["10.1"],
        ["10.2"],
        ["10.3", "10.4"],
        ["10.5"],
        ["10.6", "10.7"],
        ["10.8", "10.9", "10.10"],
        ["10.11", "10.12"],
        ["10.13", "10.14"]
    ],
    "tasks": [
        {
            "id": "10.1",
            "passes": true,
            "category": "phase1-infrastructure",
            "priority": "P1",
            "description": "创建 price_u64.rs 辅助模块",
            "context": {
                "why": "需要统一的转换函数处理 f64 ↔ u64 转换，确保所有模块使用相同的精度逻辑",
                "impact": "所有后续任务依赖此模块",
                "risk": "转换函数实现错误会导致全局计费错误"
            },
            "steps": [
                {
                    "action": "创建文件",
                    "file": "crates/common/src/price_u64.rs"
                },
                {
                    "action": "定义常量 NANO_PER_DOLLAR",
                    "value": "1_000_000_000",
                    "comment": "1 USD = 10^9 纳美元"
                },
                {
                    "action": "定义常量 RATE_SCALE",
                    "value": "1_000_000_000",
                    "comment": "汇率缩放因子，9 位精度"
                },
                {
                    "action": "实现 dollars_to_nano(price: f64) -> u64",
                    "detail": "(price * NANO_PER_DOLLAR as f64).round() as u64"
                },
                {
                    "action": "实现 nano_to_dollars(nano: u64) -> f64",
                    "detail": "nano as f64 / NANO_PER_DOLLAR as f64"
                },
                {
                    "action": "实现 rate_to_scaled(rate: f64) -> u64",
                    "detail": "汇率转缩放整数"
                },
                {
                    "action": "实现 scaled_to_rate(scaled: u64) -> f64",
                    "detail": "缩放整数转汇率"
                },
                {
                    "action": "实现 calculate_cost_safe(tokens: u64, price_per_million: u64) -> u64",
                    "detail": "使用 u128 中间值避免溢出"
                },
                {
                    "action": "添加单元测试",
                    "tests": ["roundtrip", "9位精度", "溢出保护"]
                }
            ],
            "acceptance_criteria": [
                "GIVEN 价格 $3.0",
                "WHEN 调用 dollars_to_nano",
                "THEN 返回 3_000_000_000",
                "AND roundtrip 误差 < 10^-9"
            ],
            "verification": [
                "cargo test 通过",
                "test_dollars_to_nano_roundtrip 通过",
                "test_nine_decimal_precision 通过"
            ],
            "constraints": [
                "所有函数必须是纯函数（无副作用）",
                "禁止 panic，错误通过 Result 返回"
            ],
            "references": [
                "docs/plan.md 第十七章 - 辅助模块设计"
            ]
        },
        {
            "id": "10.2",
            "passes": true,
            "category": "phase1-infrastructure",
            "priority": "P1",
            "description": "在 common/lib.rs 导出 price_u64 模块",
            "context": {
                "why": "使其他 crate 能够使用转换函数",
                "impact": "所有需要价格转换的模块"
            },
            "steps": [
                {
                    "action": "修改文件",
                    "file": "crates/common/src/lib.rs"
                },
                {
                    "action": "添加模块声明",
                    "code": "pub mod price_u64;"
                },
                {
                    "action": "重新导出常用函数",
                    "code": "pub use price_u64::{dollars_to_nano, nano_to_dollars, NANO_PER_DOLLAR};"
                }
            ],
            "verification": [
                "cargo build 编译通过",
                "其他 crate 可以 use burncloud_common::dollars_to_nano"
            ],
            "dependencies": ["10.1"]
        },
        {
            "id": "10.3",
            "passes": true,
            "category": "phase2-types",
            "priority": "P1",
            "description": "修改 types.rs 价格字段 f64 → u64",
            "context": {
                "why": "核心数据结构需要使用 u64 存储价格",
                "impact": "所有使用这些结构体的模块"
            },
            "steps": [
                {
                    "action": "修改文件",
                    "file": "crates/common/src/types.rs"
                },
                {
                    "action": "修改 PriceV2 结构体",
                    "changes": [
                        "input_price: f64 → u64",
                        "output_price: f64 → u64",
                        "cache_read_input_price: Option<f64> → Option<u64>",
                        "cache_creation_input_price: Option<f64> → Option<u64>",
                        "batch_input_price: Option<f64> → Option<u64>",
                        "batch_output_price: Option<f64> → Option<u64>",
                        "priority_input_price: Option<f64> → Option<u64>",
                        "priority_output_price: Option<f64> → Option<u64>",
                        "audio_input_price: Option<f64> → Option<u64>"
                    ]
                },
                {
                    "action": "修改 TieredPrice 结构体",
                    "changes": ["input_price: f64 → u64", "output_price: f64 → u64"]
                },
                {
                    "action": "修改 ExchangeRate 结构体",
                    "changes": ["rate: f64 → u64 (缩放值)"]
                },
                {
                    "action": "修改所有 Input 结构体",
                    "detail": "PriceV2Input, TieredPriceInput 等"
                }
            ],
            "verification": [
                "cargo check 编译通过"
            ],
            "dependencies": ["10.2"]
        },
        {
            "id": "10.4",
            "passes": true,
            "category": "phase2-types",
            "priority": "P1",
            "description": "修改 pricing_config.rs 添加自定义序列化",
            "context": {
                "why": "JSON 配置文件使用浮点数，需要自定义序列化保持兼容",
                "impact": "价格导入导出功能"
            },
            "steps": [
                {
                    "action": "修改文件",
                    "file": "crates/common/src/pricing_config.rs"
                },
                {
                    "action": "修改 CurrencyPricing",
                    "changes": ["input_price: f64 → u64", "output_price: f64 → u64"]
                },
                {
                    "action": "添加自定义 serde 序列化器",
                    "detail": "序列化时 u64→f64，反序列化时 f64→u64"
                },
                {
                    "action": "使用 serde(with) 属性",
                    "example": "#[serde(serialize_with = \"serialize_nano_as_dollars\")]"
                }
            ],
            "verification": [
                "JSON 输出仍为浮点格式",
                "JSON 输入接受浮点数",
                "cargo test 通过"
            ],
            "dependencies": ["10.2"]
        },
        {
            "id": "10.5",
            "passes": true,
            "category": "phase3-database",
            "priority": "P1",
            "description": "修改 schema.rs 数据库表 REAL → BIGINT",
            "context": {
                "why": "数据库需要使用整数类型存储纳美元价格",
                "impact": "所有价格相关表",
                "risk": "迁移失败会导致数据丢失"
            },
            "steps": [
                {
                    "action": "修改文件",
                    "file": "crates/database/src/schema.rs"
                },
                {
                    "action": "修改 prices_v2 表",
                    "changes": [
                        "input_price REAL → BIGINT",
                        "output_price REAL → BIGINT",
                        "cache_read_input_price REAL → BIGINT",
                        "cache_creation_input_price REAL → BIGINT",
                        "batch_input_price REAL → BIGINT",
                        "batch_output_price REAL → BIGINT",
                        "priority_input_price REAL → BIGINT",
                        "priority_output_price REAL → BIGINT",
                        "audio_input_price REAL → BIGINT"
                    ]
                },
                {
                    "action": "修改 tiered_pricing 表",
                    "changes": ["input_price REAL → BIGINT", "output_price REAL → BIGINT"]
                },
                {
                    "action": "修改 exchange_rates 表",
                    "changes": ["rate REAL → BIGINT"]
                },
                {
                    "action": "添加数据迁移逻辑",
                    "detail": "在 Schema::init 中添加迁移：REAL * 10^9 → BIGINT"
                }
            ],
            "acceptance_criteria": [
                "GIVEN prices_v2 表有 REAL 类型数据",
                "WHEN 执行迁移",
                "THEN 数据正确转换为 BIGINT",
                "AND $3.0 → 3000000000"
            ],
            "verification": [
                "新数据库创建正确",
                "现有数据迁移正确",
                "cargo test 通过"
            ],
            "constraints": [
                "迁移必须幂等",
                "已迁移数据不重复处理"
            ],
            "rollback": {
                "sqlite": "需重建表，从备份恢复",
                "postgresql": "ALTER COLUMN TYPE REAL"
            },
            "dependencies": ["10.3"]
        },
        {
            "id": "10.6",
            "passes": true,
            "category": "phase3-database",
            "priority": "P1",
            "description": "添加数据迁移脚本",
            "context": {
                "why": "需要将现有 f64 数据安全转换为 u64"
            },
            "steps": [
                {
                    "action": "添加 SQLite 迁移",
                    "logic": [
                        "CREATE TABLE prices_v2_new (... BIGINT ...)",
                        "INSERT INTO prices_v2_new SELECT ... ROUND(price * 1000000000) ...",
                        "DROP TABLE prices_v2",
                        "ALTER TABLE prices_v2_new RENAME TO prices_v2"
                    ]
                },
                {
                    "action": "添加 PostgreSQL 迁移",
                    "logic": [
                        "ALTER TABLE prices_v2 ALTER COLUMN input_price TYPE BIGINT USING ROUND(input_price * 1000000000)::BIGINT",
                        "..."
                    ]
                },
                {
                    "action": "添加迁移检查",
                    "detail": "检查是否已迁移，避免重复执行"
                }
            ],
            "verification": [
                "迁移前后数据对比正确",
                "迁移后价格精度验证"
            ],
            "dependencies": ["10.5"]
        },
        {
            "id": "10.7",
            "passes": true,
            "category": "phase3-database",
            "priority": "P1",
            "description": "修改 database-models 结构体",
            "context": {
                "why": "数据库模型需要匹配新的 u64 类型"
            },
            "steps": [
                {
                    "action": "修改文件",
                    "file": "crates/database/crates/database-models/src/lib.rs"
                },
                {
                    "action": "修改 Price 结构体",
                    "changes": "所有价格字段 f64 → u64"
                },
                {
                    "action": "修改 PriceV2 结构体",
                    "changes": "所有价格字段 f64/u64 → u64"
                },
                {
                    "action": "修改 TieredPrice 结构体",
                    "changes": "input_price, output_price f64 → u64"
                },
                {
                    "action": "修改 calculate_cost 方法",
                    "detail": "使用 u64 运算，返回 u64"
                },
                {
                    "action": "修改所有 Input 结构体",
                    "detail": "PriceInput, PriceV2Input, TieredPriceInput"
                }
            ],
            "verification": [
                "cargo build 编译通过",
                "数据库读写测试通过"
            ],
            "dependencies": ["10.5"]
        },
        {
            "id": "10.8",
            "passes": true,
            "category": "phase4-business-logic",
            "priority": "P1",
            "description": "修改 billing.rs 计费计算使用 u64",
            "context": {
                "why": "核心计费逻辑需要使用整数运算避免精度问题",
                "impact": "所有计费场景",
                "risk": "计费计算错误导致财务损失"
            },
            "steps": [
                {
                    "action": "修改文件",
                    "file": "crates/router/src/billing.rs"
                },
                {
                    "action": "修改 CostResult 结构体",
                    "changes": ["usd_amount: f64 → u64", "local_amount: Option<f64> → Option<u64>"]
                },
                {
                    "action": "修改 AdvancedPricing 结构体",
                    "changes": "所有价格字段 f64 → u64"
                },
                {
                    "action": "修改 format_cost 函数",
                    "detail": "从 u64 转换后格式化：nano_to_dollars(amount)"
                },
                {
                    "action": "修改 calculate_tiered_cost",
                    "detail": "使用 u64 整数运算"
                },
                {
                    "action": "修改 calculate_cache_cost",
                    "detail": "使用 u64 整数运算"
                },
                {
                    "action": "修改 calculate_batch_cost",
                    "detail": "使用 u64 整数运算"
                },
                {
                    "action": "修改 calculate_priority_cost",
                    "detail": "使用 u64 整数运算"
                },
                {
                    "action": "修改 calculate_multi_currency_cost",
                    "detail": "使用 u64 整数运算"
                },
                {
                    "action": "使用 calculate_cost_safe 避免溢出",
                    "detail": "tokens * price_per_million / 1_000_000"
                }
            ],
            "acceptance_criteria": [
                "GIVEN 150K tokens, Qwen 阶梯定价",
                "WHEN 计算 cost",
                "THEN 结果 = 334800000 (纳美元) = $0.3348"
            ],
            "verification": [
                "所有 billing 测试通过",
                "计费精度验证 < 10^-9"
            ],
            "dependencies": ["10.3", "10.7"]
        },
        {
            "id": "10.9",
            "passes": true,
            "category": "phase4-business-logic",
            "priority": "P2",
            "description": "修改 exchange_rate.rs 汇率服务",
            "context": {
                "why": "汇率服务需要使用 u64 存储缩放汇率"
            },
            "steps": [
                {
                    "action": "修改文件",
                    "file": "crates/router/src/exchange_rate.rs"
                },
                {
                    "action": "修改 CachedRate 结构体",
                    "changes": ["rate: f64 → u64 (缩放值)"]
                },
                {
                    "action": "修改 convert 方法",
                    "detail": "使用整数运算，rate_to_scaled/scaled_to_rate"
                },
                {
                    "action": "修改 get_rate 方法",
                    "returns": "Option<u64>"
                },
                {
                    "action": "修改 set_rate 方法",
                    "input": "u64 (缩放值)"
                }
            ],
            "verification": [
                "cargo test exchange_rate 通过",
                "汇率转换精度验证"
            ],
            "dependencies": ["10.3"]
        },
        {
            "id": "10.10",
            "passes": true,
            "category": "phase4-business-logic",
            "priority": "P2",
            "description": "修改 price_sync.rs 边界转换",
            "context": {
                "why": "LiteLLM 源数据是 f64，需要在写入数据库边界处转换为 u64"
            },
            "steps": [
                {
                    "action": "修改文件",
                    "file": "crates/router/src/price_sync.rs"
                },
                {
                    "action": "保持 LiteLLMPrice 结构体 f64",
                    "detail": "源数据格式不变"
                },
                {
                    "action": "修改转换函数返回 u64",
                    "changes": ["to_per_million_price() → (Option<u64>, Option<u64>)"]
                },
                {
                    "action": "在 sync_from_litellm 中转换",
                    "detail": "写入数据库时调用 dollars_to_nano()"
                }
            ],
            "verification": [
                "cargo test price_sync 通过",
                "同步后数据库值为纳美元"
            ],
            "dependencies": ["10.3", "10.7"]
        },
        {
            "id": "10.11",
            "passes": true,
            "category": "phase5-cli",
            "priority": "P2",
            "description": "修改 price.rs CLI 命令",
            "context": {
                "why": "CLI 需要接受 f64 输入（用户友好），内部转换为 u64"
            },
            "steps": [
                {
                    "action": "修改文件",
                    "file": "crates/cli/src/price.rs"
                },
                {
                    "action": "输入解析保持 f64",
                    "detail": "用户输入如 --input 3.0"
                },
                {
                    "action": "存储时转换为 u64",
                    "detail": "调用 dollars_to_nano()"
                },
                {
                    "action": "显示时转换为 f64",
                    "detail": "调用 nano_to_dollars()，格式化显示"
                },
                {
                    "action": "更新所有命令处理",
                    "commands": ["set", "get", "show", "list", "import", "export"]
                }
            ],
            "verification": [
                "CLI 输入 $3.0，数据库存储 3000000000",
                "CLI 显示 $3.00/1M"
            ],
            "dependencies": ["10.8"]
        },
        {
            "id": "10.12",
            "passes": true,
            "category": "phase5-cli",
            "priority": "P2",
            "description": "修改 currency.rs CLI 命令",
            "context": {
                "why": "汇率 CLI 需要处理 f64 输入输出"
            },
            "steps": [
                {
                    "action": "修改文件",
                    "file": "crates/cli/src/currency.rs"
                },
                {
                    "action": "汇率输入保持 f64",
                    "detail": "用户输入如 --rate 7.24"
                },
                {
                    "action": "存储时转换为 u64",
                    "detail": "调用 rate_to_scaled()"
                },
                {
                    "action": "显示时转换为 f64",
                    "detail": "调用 scaled_to_rate()"
                }
            ],
            "verification": [
                "CLI 输入汇率 7.24，存储 7240000000",
                "CLI 显示 7.24"
            ],
            "dependencies": ["10.9"]
        },
        {
            "id": "10.13",
            "passes": true,
            "category": "phase6-tests",
            "priority": "P1",
            "description": "编写单元测试",
            "context": {
                "why": "确保 u64 转换和计费逻辑正确"
            },
            "steps": [
                {
                    "action": "添加 price_u64.rs 测试",
                    "tests": [
                        "test_dollars_to_nano_roundtrip",
                        "test_nine_decimal_precision",
                        "test_rate_to_scaled_roundtrip",
                        "test_calculate_cost_safe",
                        "test_overflow_protection"
                    ]
                },
                {
                    "action": "更新 billing.rs 测试",
                    "tests": [
                        "test_tiered_cost_u64",
                        "test_cache_cost_u64",
                        "test_batch_cost_u64",
                        "test_multi_currency_u64"
                    ]
                },
                {
                    "action": "添加边界测试",
                    "cases": ["0 价格", "最大价格", "溢出边界"]
                }
            ],
            "verification": [
                "cargo test 全部通过",
                "测试覆盖率 > 90%"
            ],
            "dependencies": ["10.8"]
        },
        {
            "id": "10.14",
            "passes": null,
            "category": "phase6-tests",
            "priority": "P2",
            "description": "编写集成测试和手动验证",
            "context": {
                "why": "端到端验证整个迁移流程"
            },
            "steps": [
                {
                    "action": "创建集成测试",
                    "file": "crates/tests/pricing_u64_test.rs",
                    "tests": [
                        "价格 CRUD 操作",
                        "计费精度验证",
                        "汇率转换",
                        "阶梯定价"
                    ]
                },
                {
                    "action": "手动验证脚本",
                    "steps": [
                        "创建演示数据库",
                        "设置价格验证存储",
                        "设置极小价格测试精度"
                    ]
                }
            ],
            "verification": [
                "集成测试通过",
                "手动验证: $2.5 → 2500000000",
                "手动验证: $0.000000123 → 123"
            ],
            "dependencies": ["10.13"]
        }
    ],
    "dependencies": {
        "10.2": ["10.1"],
        "10.3": ["10.2"],
        "10.4": ["10.2"],
        "10.5": ["10.3"],
        "10.6": ["10.5"],
        "10.7": ["10.5"],
        "10.8": ["10.3", "10.7"],
        "10.9": ["10.3"],
        "10.10": ["10.3", "10.7"],
        "10.11": ["10.8"],
        "10.12": ["10.9"],
        "10.13": ["10.8"],
        "10.14": ["10.13"]
    },
    "test_data": {
        "price_conversions": {
            "description": "f64 到 u64 纳美元转换示例",
            "examples": [
                {"f64": 3.0, "u64": 3000000000, "display": "$3.00"},
                {"f64": 0.15, "u64": 150000000, "display": "$0.15"},
                {"f64": 0.00015, "u64": 150000, "display": "$0.00015"},
                {"f64": 0.000000001, "u64": 1, "display": "$0.000000001"},
                {"f64": 1.000000001, "u64": 1000000001, "display": "$1.000000001"}
            ]
        },
        "exchange_rate_conversions": {
            "description": "汇率 f64 到 u64 缩放转换示例",
            "examples": [
                {"f64": 7.24, "u64": 7240000000},
                {"f64": 0.138, "u64": 138000000},
                {"f64": 1.08, "u64": 1080000000}
            ]
        },
        "billing_calculations": {
            "description": "计费计算示例（纳美元）",
            "examples": [
                {
                    "tokens": 1000000,
                    "price_per_million": 3000000000,
                    "expected_cost_nano": 3000000000,
                    "expected_cost_usd": 3.0
                },
                {
                    "tokens": 150000,
                    "price_per_million": 1200000000,
                    "expected_cost_nano": 180000000,
                    "expected_cost_usd": 0.18
                }
            ]
        },
        "tiered_billing": {
            "description": "阶梯计费示例（Qwen 150K tokens）",
            "tokens": 150000,
            "tiers": [
                {"tier_start": 0, "tier_end": 32000, "price": 1200000000},
                {"tier_start": 32000, "tier_end": 128000, "price": 2400000000},
                {"tier_start": 128000, "tier_end": 252000, "price": 3000000000}
            ],
            "calculation": [
                {"range": "0-32K", "tokens": 32000, "price": 1200000000, "cost": 38400000},
                {"range": "32K-128K", "tokens": 96000, "price": 2400000000, "cost": 230400000},
                {"range": "128K-150K", "tokens": 22000, "price": 3000000000, "cost": 66000000}
            ],
            "total_cost_nano": 334800000,
            "total_cost_usd": 0.3348
        }
    },
    "api_contracts": {
        "PriceU64": {
            "dollars_to_nano": {
                "input": {"price": "f64"},
                "output": "u64",
                "precision": "9 decimal places"
            },
            "nano_to_dollars": {
                "input": {"nano": "u64"},
                "output": "f64"
            },
            "calculate_cost_safe": {
                "input": {
                    "tokens": "u64",
                    "price_per_million_nano": "u64"
                },
                "output": "u64",
                "overflow_protection": "uses u128 intermediate"
            }
        },
        "BillingService": {
            "calculate_cost": {
                "input": {
                    "tokens": "u64",
                    "price_nano": "u64"
                },
                "output": "u64 (nanodollars)",
                "hot_path": true
            }
        }
    },
    "summary": {
        "critical_path": [
            "10.1",
            "10.2",
            "10.3",
            "10.5",
            "10.7",
            "10.8",
            "10.13"
        ],
        "parallelizable": [
            ["10.3", "10.4"],
            ["10.6", "10.7"],
            ["10.8", "10.9", "10.10"],
            ["10.11", "10.12"],
            ["10.13", "10.14"]
        ],
        "risk_areas": [
            {
                "area": "数据迁移",
                "risk": "迁移失败导致数据丢失或精度错误",
                "mitigation": "备份数据库，迁移后验证对比"
            },
            {
                "area": "溢出问题",
                "risk": "大 token 数量 × 高价格可能溢出",
                "mitigation": "使用 u128 中间值，添加边界检查"
            },
            {
                "area": "API 兼容性",
                "risk": "JSON 格式变化影响客户端",
                "mitigation": "自定义序列化器，输出保持浮点格式"
            },
            {
                "area": "CLI 用户体验",
                "risk": "用户不理解纳美元单位",
                "mitigation": "CLI 显示浮点格式，隐藏内部实现"
            }
        ],
        "definition_of_done": [
            "所有 P1 任务完成并通过验收",
            "所有单元测试通过",
            "数据库迁移验证正确",
            "CLI 功能手动验证",
            "计费精度 < 10^-9"
        ]
    }
}
