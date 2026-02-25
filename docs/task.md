[
  {
    "category": "error-handling",
    "description": "消除 router/src/lib.rs 中的 .unwrap() 调用",
    "steps": [
      "修复 Response::builder() 链中的 .unwrap()，改用 unwrap_or_else 或 ? 操作符",
      "修复 serde_json 序列化中的 .unwrap()，使用 ? 传播错误",
      "修复流式响应处理中的 .unwrap()",
      "运行 cargo clippy 验证无警告"
    ],
    "passes": true
  },
  {
    "category": "error-handling",
    "description": "消除 adaptor 模块中的 .unwrap() 和 .expect()",
    "steps": [
      "修复 adaptor/claude.rs:65 的 SystemTime::now().unwrap()",
      "修复 adaptor/gemini.rs:99,159 的 .unwrap()",
      "修复 adaptor/vertex.rs:255,313,383,462 的 .expect()",
      "使用 thiserror 定义结构化错误并传播"
    ],
    "passes": true
  },
  {
    "category": "error-handling",
    "description": "消除 response_parser.rs 中的 .unwrap()",
    "steps": [
      "修复 response_parser.rs:543-545 的 HeaderValue::parse().unwrap()",
      "修复 response_parser.rs:557-559, 570-572 的类似问题",
      "使用 .parse().unwrap_or_else() 或定义默认值"
    ],
    "passes": true
  },
  {
    "category": "deps",
    "description": "移除库代码中的 anyhow 依赖",
    "steps": [
      "从 database-router/Cargo.toml 移除 anyhow",
      "从 database-user/Cargo.toml 移除 anyhow",
      "从 service-user/Cargo.toml 移除 anyhow",
      "从 service-redis/Cargo.toml 移除 anyhow",
      "使用 thiserror 替代错误定义"
    ],
    "passes": true
  },
  {
    "category": "deps",
    "description": "迁移 service-user 使用 workspace 依赖",
    "steps": [
      "在根 Cargo.toml 添加 bcrypt 和 jsonwebtoken 到 workspace.dependencies",
      "将 bcrypt = \"0.17.1\" 改为 bcrypt.workspace = true",
      "将 jsonwebtoken = \"9.3\" 改为 jsonwebtoken.workspace = true",
      "运行 cargo build 验证依赖解析正确"
    ],
    "passes": true
  },
  {
    "category": "numeric",
    "description": "将金额字段从 f64 迁移到 i64 纳美元",
    "steps": [
      "在 common/src/types.rs 定义纳美元转换工具函数",
      "修改 database-user/src/lib.rs 的 update_balance 使用 i64",
      "修改 service-user/src/lib.rs 的 SIGNUP_BONUS 为 i64",
      "修改 server/src/api/user.rs 的 amount 字段为 i64",
      "修改 client-shared/services 中的 balance/amount 字段",
      "更新相关 API 和数据库迁移脚本"
    ],
    "passes": true
  },
  {
    "category": "database",
    "description": "修复 database-router SQL PostgreSQL/SQLite 兼容性",
    "steps": [
      "为常用查询添加 is_postgres 检查",
      "将 ? 占位符替换为 $1, $2... (PostgreSQL) 或 ? (SQLite)",
      "封装 SQL 生成逻辑到辅助函数",
      "测试 SQLite 和 PostgreSQL 兼容性"
    ],
    "passes": true
  },
  {
    "category": "database",
    "description": "修复 database-user SQL PostgreSQL/SQLite 兼容性",
    "steps": [
      "修复 recharges 查询的占位符兼容性",
      "修复 update_balance 等方法的 SQL 语法",
      "运行 cargo test 验证"
    ],
    "passes": true
  },
  {
    "category": "tech-debt",
    "description": "处理生产代码中的 TODO 注释",
    "steps": [
      "评估 database-router/src/lib.rs:701 的 TODO (用户配额集成)",
      "评估 router/src/limiter.rs:60 的 TODO (用户级限流)",
      "评估 router/src/adaptor/gemini.rs:85 的 TODO (错误处理)",
      "评估 router/src/lib.rs:397 的 TODO (tiktoken 集成)",
      "评估 router/src/notification.rs:219 的 TODO (邮件发送)",
      "评估 server/src/api/channel.rs:75 的 TODO (分页)",
      "决定：实现功能 或 创建 GitHub Issue 跟踪"
    ],
    "passes": false
  },
  {
    "category": "verify",
    "description": "运行完整验证流程",
    "steps": [
      "运行 cargo fmt 格式化代码",
      "运行 cargo clippy -- -D warnings 确认无警告",
      "运行 cargo test 确认所有测试通过",
      "确认无 .unwrap()/.expect() 在生产代码: grep -rn '.unwrap()' crates/",
      "确认无 f64 金额字段: grep -rn 'balance.*f64\\|amount.*f64' crates/",
      "确认所有依赖使用 workspace: grep -rn '= \"[0-9]' crates/*/Cargo.toml"
    ],
    "passes": false
  }
]
