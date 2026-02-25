[
  {
    "category": "build",
    "description": "修复 lib.rs 重复导出语法错误",
    "steps": [
      "定位 crates/router/src/lib.rs:40-41 的语法错误",
      "修正为 pub use proxy_logic::{proxy_logic, handle_response_with_token_parsing};",
      "运行 cargo build 验证编译通过"
    ],
    "passes": true
  },
  {
    "category": "build",
    "description": "修复 AppState 缺少右尖括号语法错误",
    "steps": [
      "修复 crates/router/src/lib.rs:82 缺少的 >",
      "修复 crates/router/src/proxy_logic.rs:21 缺少的 >",
      "运行 cargo build 验证编译通过"
    ],
    "passes": false
  },
  {
    "category": "refactor",
    "description": "消除 AppState 重复定义",
    "steps": [
      "保留 crates/router/src/state.rs 中的 AppState 作为唯一定义",
      "在 lib.rs 中改为 pub use state::AppState;",
      "删除 proxy_logic.rs 中的重复 AppState 定义",
      "在 proxy_logic.rs 中添加 use crate::state::AppState;",
      "运行 cargo build 和 cargo clippy 验证"
    ],
    "passes": false
  },
  {
    "category": "refactor",
    "description": "消除 Price 类型重复定义",
    "steps": [
      "保留 crates/common/src/types.rs 中的 Price 和 PriceInput 定义",
      "在 database-models/price.rs 中删除重复的结构体定义",
      "在 database-models/price.rs 中添加 use burncloud_common::types::{Price, PriceInput};",
      "保留 PriceModel 的方法实现（get, upsert 等）",
      "运行 cargo build 验证"
    ],
    "passes": false
  },
  {
    "category": "refactor",
    "description": "消除 TieredPrice 类型重复定义",
    "steps": [
      "保留 crates/common/src/types.rs 中的 TieredPrice 和 TieredPriceInput 定义",
      "在 database-models/tiered_price.rs 中删除重复的结构体定义",
      "在 database-models/tiered_price.rs 中添加 use burncloud_common::types::{TieredPrice, TieredPriceInput};",
      "保留 TieredPriceModel 的方法实现",
      "运行 cargo build 验证"
    ],
    "passes": false
  },
  {
    "category": "deps",
    "description": "添加缺失的 workspace 依赖到根 Cargo.toml",
    "steps": [
      "在根 Cargo.toml 的 [workspace.dependencies] 中添加 bcrypt = \"0.15\"",
      "添加 futures = \"0.3\"",
      "添加 regex = \"1\"",
      "添加 mockito = \"1.7\"",
      "添加 tempfile = \"3\""
    ],
    "passes": false
  },
  {
    "category": "deps",
    "description": "迁移 router/Cargo.toml 使用 workspace 依赖",
    "steps": [
      "将 futures = \"0.3.31\" 改为 futures.workspace = true",
      "将 regex = \"1.12.3\" 改为 regex.workspace = true",
      "将 mockito = \"1.7.1\" 改为 mockito.workspace = true",
      "将 tempfile = \"3\" 改为 tempfile.workspace = true",
      "运行 cargo build 验证依赖解析正确"
    ],
    "passes": false
  },
  {
    "category": "deps",
    "description": "迁移 common/Cargo.toml 使用 workspace 依赖",
    "steps": [
      "将 bcrypt = \"0.15\" 改为 bcrypt.workspace = true",
      "运行 cargo build 验证"
    ],
    "passes": false
  },
  {
    "category": "tech-debt",
    "description": "处理 service-inference 中的 TODO 遗留",
    "steps": [
      "评估 crates/service/crates/service-inference/src/lib.rs:103 的 TODO",
      "决定：实现健康检查 或 移除 TODO 注释",
      "运行 cargo clippy 确认无警告"
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
      "确认无重复定义: grep -r 'pub struct AppState' crates/",
      "确认无硬编码依赖: grep -r '= \"' crates/*/Cargo.toml"
    ],
    "passes": false
  }
]
