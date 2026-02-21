{
    "meta": {
        "title": "区域定价与双币扣费 - 实施任务清单",
        "version": "1.0",
        "source": "docs/plan.md 第十八章",
        "priority_order": [
            "P1",
            "P2"
        ],
        "estimated_phases": 6,
        "total_tasks": 15,
        "estimated_effort": "5-7 days",
        "task_status_field": {
            "passes": "null=未开始, true=已完成, false=失败需重试"
        }
    },
    "context": {
        "problem": "国内模型（如 Qwen）有人民币价格，但系统可能把它们硬转成美元。当汇率变动时，硬转换的价格就不准确。同一个模型同时有 USD 和 CNY 两种价格的设计是错误的。",
        "goal": "实现区域定价和双币扣费：同一渠道同一模型只能有一种货币定价，国内渠道用 CNY，海外渠道用 USD，绝对不能把人民币价格硬转成美元。",
        "core_principle": "同一个渠道同一个模型只能有一种货币定价",
        "constraints": [
            "国内渠道 → 使用人民币定价（CNY）",
            "海外渠道 → 使用美元定价（USD）",
            "绝对不能把人民币价格硬转成美元",
            "余额使用 u64（无符号）保证非负",
            "prices_v2 正式替换 prices 表",
            "移除废弃的 quota 字段"
        ],
        "negative_constraints": [
            "不要在同一区域为同一模型设置两种货币价格",
            "不要把 CNY 价格硬转成 USD",
            "不要在扣费前不检查余额充足性",
            "不要使用 i64 作为余额类型（应使用 u64）"
        ],
        "security": {
            "input_validation": "余额必须 >= 0",
            "overflow_protection": "使用 u128 中间值计算汇率转换",
            "balance_check": "请求前必须预判余额是否充足"
        },
        "metrics": {
            "balance_precision": "u64 nanodollars (9位小数)",
            "pre_check_latency": "< 5ms (一次数据库查询)",
            "cross_currency_accuracy": "汇率转换精度 10^-9"
        }
    },
    "execution_order": [
        ["18.1", "18.2"],
        ["18.3", "18.4"],
        ["18.5"],
        ["18.6", "18.7", "18.8"],
        ["18.9", "18.10", "18.11"],
        ["18.12", "18.13"],
        ["18.14", "18.15"]
    ],
    "tasks": [
        {
            "id": "18.1",
            "passes": true,
            "category": "phase1-database",
            "priority": "P1",
            "description": "修改 users 表 - 移除 quota，添加双币钱包",
            "context": {
                "why": "废弃的 quota 字段需要移除，改用双币钱包支持区域定价",
                "impact": "所有用户余额逻辑的基础",
                "risk": "数据迁移需谨慎，避免丢失用户余额"
            },
            "steps": [
                {
                    "action": "添加 balance_usd 字段",
                    "file": "crates/database/src/schema.rs",
                    "detail": "ALTER TABLE users ADD COLUMN balance_usd BIGINT UNSIGNED DEFAULT 0"
                },
                {
                    "action": "添加 balance_cny 字段",
                    "file": "crates/database/src/schema.rs",
                    "detail": "ALTER TABLE users ADD COLUMN balance_cny BIGINT UNSIGNED DEFAULT 0"
                },
                {
                    "action": "添加 preferred_currency 字段",
                    "file": "crates/database/src/schema.rs",
                    "detail": "ALTER TABLE users ADD COLUMN preferred_currency VARCHAR(10) DEFAULT 'USD'"
                },
                {
                    "action": "移除 quota 字段",
                    "file": "crates/database/src/schema.rs",
                    "detail": "ALTER TABLE users DROP COLUMN quota"
                },
                {
                    "action": "移除 used_quota 字段",
                    "file": "crates/database/src/schema.rs",
                    "detail": "ALTER TABLE users DROP COLUMN used_quota"
                }
            ],
            "acceptance_criteria": [
                "users 表包含 balance_usd, balance_cny, preferred_currency 字段",
                "quota 和 used_quota 字段已移除",
                "现有用户数据迁移正确"
            ]
        },
        {
            "id": "18.2",
            "passes": true,
            "category": "phase1-database",
            "priority": "P1",
            "description": "修改 router_tokens 表 - 移除 quota 字段",
            "context": {
                "why": "token 级别的 quota 已废弃，统一使用用户级双币钱包",
                "impact": "token 配额检查逻辑",
                "risk": "需要更新所有使用 quota 的代码"
            },
            "steps": [
                {
                    "action": "移除 quota_limit 字段",
                    "file": "crates/database/src/schema.rs",
                    "detail": "ALTER TABLE router_tokens DROP COLUMN quota_limit"
                },
                {
                    "action": "移除 used_quota 字段",
                    "file": "crates/database/src/schema.rs",
                    "detail": "ALTER TABLE router_tokens DROP COLUMN used_quota"
                },
                {
                    "action": "移除 unlimited_quota 字段",
                    "file": "crates/database/src/schema.rs",
                    "detail": "ALTER TABLE router_tokens DROP COLUMN unlimited_quota"
                }
            ],
            "acceptance_criteria": [
                "router_tokens 表不再有 quota 相关字段",
                "现有 token 数据迁移正确"
            ]
        },
        {
            "id": "18.3",
            "passes": true,
            "category": "phase2-prices",
            "priority": "P1",
            "description": "修改 prices_v2 表约束 - UNIQUE(model, region)",
            "context": {
                "why": "确保同一模型在同一区域只有一种货币价格",
                "impact": "价格存储的核心约束",
                "risk": "需要清理可能的重复数据"
            },
            "steps": [
                {
                    "action": "清理重复数据",
                    "detail": "DELETE FROM prices_v2 WHERE id NOT IN (SELECT MIN(id) FROM prices_v2 GROUP BY model, region)"
                },
                {
                    "action": "修改唯一约束",
                    "detail": "UNIQUE(model, currency, region) → UNIQUE(model, region)"
                },
                {
                    "action": "更新 upsert 逻辑",
                    "file": "crates/database/crates/database-models/src/lib.rs",
                    "detail": "ON CONFLICT(model, region) DO UPDATE SET ..."
                }
            ],
            "acceptance_criteria": [
                "prices_v2 约束为 UNIQUE(model, region)",
                "无法为同一模型+区域插入两种货币价格"
            ]
        },
        {
            "id": "18.4",
            "passes": true,
            "category": "phase2-prices",
            "priority": "P1",
            "description": "废弃旧 prices 表",
            "context": {
                "why": "prices_v2 已成熟，正式替换旧表",
                "impact": "所有使用 PriceModel 的代码需要迁移",
                "risk": "需要确保所有调用点都已迁移"
            },
            "steps": [
                {
                    "action": "重命名旧表",
                    "file": "crates/database/src/schema.rs",
                    "detail": "ALTER TABLE prices RENAME TO prices_deprecated"
                },
                {
                    "action": "添加迁移注释",
                    "detail": "-- Deprecated: Use prices_v2 instead"
                }
            ],
            "acceptance_criteria": [
                "prices 表已重命名为 prices_deprecated",
                "所有代码使用 PriceV2Model"
            ]
        },
        {
            "id": "18.5",
            "passes": true,
            "category": "phase3-types",
            "priority": "P1",
            "description": "更新 User 和 Token 类型定义",
            "context": {
                "why": "类型定义需要与数据库结构同步",
                "impact": "所有使用 User 和 Token 的代码",
                "risk": "编译错误需要逐一修复"
            },
            "steps": [
                {
                    "action": "更新 User 结构体",
                    "file": "crates/common/src/types.rs",
                    "detail": "移除 quota/used_quota，添加 balance_usd/balance_cny/preferred_currency (u64 类型)"
                },
                {
                    "action": "更新 Token 结构体",
                    "file": "crates/common/src/types.rs",
                    "detail": "移除 quota_limit/used_quota/unlimited_quota"
                },
                {
                    "action": "更新 DbUser 结构体",
                    "file": "crates/database/crates/database-user/src/lib.rs",
                    "detail": "与 User 结构体同步"
                }
            ],
            "acceptance_criteria": [
                "User 结构体包含双币字段",
                "Token 结构体不包含 quota 字段",
                "编译通过"
            ]
        },
        {
            "id": "18.6",
            "passes": true,
            "category": "phase4-billing",
            "priority": "P1",
            "description": "实现 PriceV2Model::get_by_model_region",
            "context": {
                "why": "简化价格查询，一个区域只有一种货币",
                "impact": "所有价格查询逻辑",
                "risk": "需要处理 region 为 NULL 的情况"
            },
            "steps": [
                {
                    "action": "实现 get_by_model_region 函数",
                    "file": "crates/database/crates/database-models/src/lib.rs",
                    "detail": "直接查询 (model, region)，返回 PriceV2"
                },
                {
                    "action": "实现 region fallback 逻辑",
                    "detail": "region 未找到时回退到 region=NULL 的通用价格"
                }
            ],
            "acceptance_criteria": [
                "可以根据 model + region 获取价格",
                "region 不存在时正确回退",
                "单元测试通过"
            ]
        },
        {
            "id": "18.7",
            "passes": true,
            "category": "phase4-billing",
            "priority": "P1",
            "description": "实现双币扣费逻辑 deduct_dual_currency",
            "context": {
                "why": "根据模型区域优先扣对应币种",
                "impact": "核心扣费逻辑",
                "risk": "汇率转换精度问题"
            },
            "steps": [
                {
                    "action": "实现 deduct_cny 函数",
                    "file": "crates/database/crates/database-router/src/lib.rs",
                    "detail": "扣减 CNY 余额，检查充足性"
                },
                {
                    "action": "实现 deduct_usd 函数",
                    "file": "crates/database/crates/database-router/src/lib.rs",
                    "detail": "扣减 USD 余额，检查充足性"
                },
                {
                    "action": "实现 deduct_dual_currency 函数",
                    "file": "crates/database/crates/database-router/src/lib.rs",
                    "detail": "根据 cost_currency 决定优先扣减顺序，不足时换汇"
                },
                {
                    "action": "移除 deduct_quota 函数",
                    "file": "crates/database/crates/database-router/src/lib.rs",
                    "detail": "已废弃，使用 deduct_dual_currency 替代"
                }
            ],
            "acceptance_criteria": [
                "CNY 模型优先扣 CNY 余额",
                "USD 模型优先扣 USD 余额",
                "不足时正确换汇",
                "使用 u128 中间值避免溢出"
            ]
        },
        {
            "id": "18.8",
            "passes": null,
            "category": "phase4-billing",
            "priority": "P2",
            "description": "实现余额预判逻辑（基于 max_tokens）",
            "context": {
                "why": "请求前预判余额，防止超额消费",
                "impact": "用户请求流程",
                "risk": "预判过于保守可能拒绝有效请求"
            },
            "steps": [
                {
                    "action": "提取 max_tokens 参数",
                    "detail": "从请求体获取 max_tokens，无则使用模型默认值"
                },
                {
                    "action": "计算最大预估费用",
                    "detail": "(estimated_input + max_output) × 价格"
                },
                {
                    "action": "检查双币余额总和",
                    "detail": "primary + secondary × rate >= max_cost"
                },
                {
                    "action": "返回 402 错误",
                    "detail": "余额不足时返回 Insufficient balance 错误"
                }
            ],
            "acceptance_criteria": [
                "正确提取 max_tokens",
                "正确计算预估费用",
                "余额不足时正确拒绝"
            ]
        },
        {
            "id": "18.9",
            "passes": null,
            "category": "phase5-router",
            "priority": "P1",
            "description": "修改 proxy_logic 返回 pricing_region",
            "context": {
                "why": "需要将渠道的区域信息传递到扣费逻辑",
                "impact": "路由层核心函数签名",
                "risk": "调用点需要更新"
            },
            "steps": [
                {
                    "action": "修改函数签名",
                    "file": "crates/router/src/lib.rs",
                    "detail": "-> (Response, Option<String>, StatusCode, Option<String>) // 新增 pricing_region"
                },
                {
                    "action": "在 channel 选择时获取 pricing_region",
                    "detail": "let pricing_region = channel.pricing_region.clone()"
                },
                {
                    "action": "更新所有调用点",
                    "detail": "接收并使用 pricing_region"
                }
            ],
            "acceptance_criteria": [
                "proxy_logic 返回 pricing_region",
                "所有调用点已更新",
                "编译通过"
            ]
        },
        {
            "id": "18.10",
            "passes": null,
            "category": "phase5-router",
            "priority": "P1",
            "description": "集成区域价格查询",
            "context": {
                "why": "使用 PriceV2Model::get_by_model_region 获取区域价格",
                "impact": "计费逻辑",
                "risk": "价格获取失败需要处理"
            },
            "steps": [
                {
                    "action": "调用 get_by_model_region",
                    "file": "crates/router/src/lib.rs",
                    "detail": "PriceV2Model::get_by_model_region(&state.db, model, pricing_region.as_deref()).await?"
                },
                {
                    "action": "提取 cost_currency",
                    "detail": "let cost_currency = price.currency; // USD 或 CNY"
                },
                {
                    "action": "实现价格回退逻辑",
                    "detail": "region 价格不存在时回退到通用价格"
                }
            ],
            "acceptance_criteria": [
                "正确获取区域价格",
                "正确识别货币类型",
                "回退逻辑正确"
            ]
        },
        {
            "id": "18.11",
            "passes": null,
            "category": "phase5-router",
            "priority": "P1",
            "description": "集成双币扣费调用",
            "context": {
                "why": "请求完成后调用双币扣费逻辑",
                "impact": "实际扣费流程",
                "risk": "扣费失败需要记录日志"
            },
            "steps": [
                {
                    "action": "获取汇率",
                    "detail": "state.exchange_rate_service.get_rate_nano(USD, CNY)"
                },
                {
                    "action": "调用 deduct_dual_currency",
                    "file": "crates/router/src/lib.rs",
                    "detail": "RouterDatabase::deduct_dual_currency(&state.db, &user_id, cost_nano, cost_currency, exchange_rate).await?"
                },
                {
                    "action": "处理扣费失败",
                    "detail": "记录日志，但不影响响应返回"
                }
            ],
            "acceptance_criteria": [
                "正确获取汇率",
                "正确调用双币扣费",
                "扣费失败正确处理"
            ]
        },
        {
            "id": "18.12",
            "passes": null,
            "category": "phase6-config",
            "priority": "P2",
            "description": "修正价格配置示例",
            "context": {
                "why": "当前示例错误地展示了同一模型同时有两种货币价格",
                "impact": "用户配置参考",
                "risk": "用户可能复制错误配置"
            },
            "steps": [
                {
                    "action": "更新 pricing.example.json",
                    "file": "docs/config/pricing.example.json",
                    "detail": "每个区域只有一种货币"
                },
                {
                    "action": "添加配置说明",
                    "detail": "说明 cn 区域用 CNY，international 区域用 USD"
                }
            ],
            "acceptance_criteria": [
                "示例配置正确",
                "每个区域只有一种货币"
            ]
        },
        {
            "id": "18.13",
            "passes": null,
            "category": "phase6-config",
            "priority": "P2",
            "description": "更新 CLI 价格命令",
            "context": {
                "why": "CLI 需要支持 --region 和 --currency 参数",
                "impact": "用户设置价格的方式",
                "risk": "需要验证同一区域只能设置一种货币"
            },
            "steps": [
                {
                    "action": "验证 region + currency 约束",
                    "file": "crates/cli/src/price.rs",
                    "detail": "同一区域已存在不同货币价格时拒绝"
                },
                {
                    "action": "更新帮助文档",
                    "detail": "说明区域定价规则"
                }
            ],
            "acceptance_criteria": [
                "CLI 正确拒绝冲突的价格设置",
                "帮助文档清晰"
            ]
        },
        {
            "id": "18.14",
            "passes": null,
            "category": "testing",
            "priority": "P1",
            "description": "编写单元测试",
            "context": {
                "why": "验证核心逻辑正确性",
                "impact": "代码质量保证",
                "risk": "测试覆盖不全可能遗漏 bug"
            },
            "steps": [
                {
                    "action": "测试 one_currency_per_region",
                    "detail": "同一模型+区域插入两种货币 → 应该失败"
                },
                {
                    "action": "测试 balance_check_with_max_tokens",
                    "detail": "余额不足时拒绝请求"
                },
                {
                    "action": "测试 dual_currency_cn_model",
                    "detail": "CNY 模型扣 CNY 余额"
                },
                {
                    "action": "测试 cross_currency_deduction",
                    "detail": "CNY 不足时用 USD 补足"
                }
            ],
            "acceptance_criteria": [
                "所有单元测试通过",
                "测试覆盖核心场景"
            ]
        },
        {
            "id": "18.15",
            "passes": null,
            "category": "testing",
            "priority": "P1",
            "description": "编写集成测试",
            "context": {
                "why": "验证端到端流程",
                "impact": "系统可靠性",
                "risk": "环境配置复杂"
            },
            "steps": [
                {
                    "action": "测试设置国内价格",
                    "detail": "burncloud price set qwen-max --input 0.359 --output 1.434 --currency CNY --region cn"
                },
                {
                    "action": "测试拒绝冲突价格",
                    "detail": "同一 region 设置不同货币 → 错误"
                },
                {
                    "action": "测试设置海外价格",
                    "detail": "burncloud price set qwen-max --input 1.2 --output 6.0 --currency USD --region international"
                },
                {
                    "action": "测试双币扣费流程",
                    "detail": "充值 CNY → 消费 cn 模型 → CNY 余额减少"
                }
            ],
            "acceptance_criteria": [
                "所有集成测试通过",
                "端到端流程正确"
            ]
        }
    ],
    "risks": {
        "items": [
            {
                "area": "数据迁移",
                "risk": "用户余额迁移错误",
                "mitigation": "备份、验证、回滚方案"
            },
            {
                "area": "约束变更",
                "risk": "现有价格数据违反新约束",
                "mitigation": "迁移前清理重复数据"
            },
            {
                "area": "汇率转换",
                "risk": "精度丢失或溢出",
                "mitigation": "使用 u128 中间值"
            },
            {
                "area": "余额预判",
                "risk": "预判过于保守拒绝有效请求",
                "mitigation": "使用 max_tokens 作为合理上限"
            }
        ]
    },
    "definition_of_done": [
        "所有 P1 任务完成并通过验收",
        "所有单元测试和集成测试通过",
        "数据库迁移验证正确",
        "CLI 功能手动验证",
        "双币扣费精度 < 10^-9"
    ]
}
