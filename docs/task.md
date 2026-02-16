[
    {
        "category": "token-cli",
        "description": "Task 1.1: 添加 TokenInput 结构体",
        "passes": true,
        "steps": [
            "文件: crates/database/crates/database-models/src/lib.rs",
            "定义 TokenInput 结构体包含字段: user_id, name, remain_quota, unlimited_quota, expired_time",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 1.2: 实现 TokenModel::create",
        "passes": true,
        "steps": [
            "文件: crates/database/crates/database-models/src/lib.rs",
            "生成唯一的 Token key (sk- 前缀 + 48位随机字符)",
            "插入数据库并返回创建的 Token",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 1.3: 实现 TokenModel::get_by_key",
        "passes": true,
        "steps": [
            "文件: crates/database/crates/database-models/src/lib.rs",
            "根据 key 查询 Token",
            "返回 Option<Token>",
            "验证: 支持 SQLite 和 PostgreSQL"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 1.4: 实现 TokenModel::list",
        "passes": true,
        "steps": [
            "文件: crates/database/crates/database-models/src/lib.rs",
            "支持分页查询 (limit, offset)",
            "支持按 user_id 过滤",
            "返回 Vec<Token>"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 1.5: 实现 TokenModel::update",
        "passes": true,
        "steps": [
            "文件: crates/database/crates/database-models/src/lib.rs",
            "根据 key 更新 Token 属性 (name, remain_quota, status, expired_time)",
            "返回更新结果",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 1.6: 实现 TokenModel::delete",
        "passes": true,
        "steps": [
            "文件: crates/database/crates/database-models/src/lib.rs",
            "根据 key 删除 Token",
            "返回删除结果",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 2.1: 创建 token.rs 模块",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/token.rs (新建)",
            "创建 handle_token_command 函数框架",
            "函数签名与 price.rs 一致",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 2.2: 实现 token list 命令",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/token.rs",
            "支持 --limit 和 --offset 参数",
            "支持 --user-id 过滤",
            "表格显示: Key, Name, User, Quota, Status, Expired"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 2.3: 实现 token create 命令",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/token.rs",
            "必需参数: --user-id",
            "可选参数: --name, --quota, --unlimited, --expired",
            "输出生成的 Token key"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 2.4: 实现 token update 命令",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/token.rs",
            "参数: key (位置参数)",
            "可选参数: --name, --quota, --status",
            "输出更新确认"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 2.5: 实现 token delete 命令",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/token.rs",
            "参数: key (位置参数)",
            "支持 -y, --yes 跳过确认",
            "默认需要用户确认"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 3.1: 在 lib.rs 中导出 token 模块",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/lib.rs",
            "添加 pub mod token;",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 3.2: 在 commands.rs 中添加 token 子命令定义",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/commands.rs",
            "添加 token 子命令定义",
            "包含 list, create, update, delete 子命令",
            "验证: 参数定义正确"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 3.3: 在 commands.rs 中添加 token 命令处理",
        "passes": true,
        "steps": [
            "文件: crates/cli/src/commands.rs",
            "添加 Some((\"token\", sub_m)) => { ... } 处理逻辑",
            "数据库连接正确管理",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 4.1: 确认 rand crate 依赖",
        "passes": true,
        "steps": [
            "文件: Cargo.toml",
            "检查 rand crate 是否在依赖中",
            "如无则添加",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "token-cli",
        "description": "Task 4.2: 验证 tokens 表结构",
        "passes": true,
        "steps": [
            "检查数据库迁移脚本中 tokens 表定义",
            "确认包含所有字段: id, user_id, key, status, name, remain_quota, unlimited_quota, used_quota, created_time, accessed_time, expired_time",
            "验证: 表结构完整"
        ]
    }
]
