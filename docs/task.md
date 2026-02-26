[
  {
    "category": "cli-user-register",
    "description": "P0: 实现 user register 命令",
    "steps": [
      "新建文件 crates/cli/src/user.rs",
      "添加模块导入: use anyhow::Result; use burncloud_database::Database; use clap::ArgMatches;",
      "实现 cmd_user_register 函数，读取 --username, --password, --email 参数",
      "调用 burncloud_database_user::DbUser::create 创建用户",
      "输出创建成功的用户 ID",
      "实现 handle_user_command 路由函数",
      "在 crates/cli/src/lib.rs 添加 mod user; 和 pub use user::*;",
      "在 crates/cli/src/commands.rs 的 Command::new(\"user\") 下添加 register 子命令",
      "register 子命令参数: --username (required), --password (required), --email (optional)",
      "在 handle_command 的 match 中添加 Some((\"user\", sub_m)) => handle_user_command(&db, sub_m).await",
      "编译验证: cargo build -p burncloud",
      "运行测试: ./target/debug/burncloud user register --help"
    ],
    "passes": true
  },
  {
    "category": "cli-user-login",
    "description": "P0: 实现 user login 命令",
    "steps": [
      "在 user.rs 添加 cmd_user_login 函数",
      "读取 --username, --password 参数",
      "调用 burncloud_database_user::DbUser::verify_password 验证凭据",
      "成功时输出用户信息和 token，失败时输出错误",
      "在 commands.rs 的 user 子命令下添加 login 子命令",
      "login 子命令参数: --username (required), --password (required)",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-user-list",
    "description": "P0: 实现 user list 命令",
    "steps": [
      "在 user.rs 添加 cmd_user_list 函数",
      "读取 --limit (默认100), --offset (默认0), --format (默认table) 参数",
      "调用 burncloud_database_user::DbUser::list 查询用户列表",
      "table 格式: 输出表头 ID, Username, Email, Balance_USD, Balance_CNY, Status",
      "json 格式: 输出 JSON 数组",
      "在 commands.rs 的 user 子命令下添加 list 子命令",
      "list 子命令参数: --limit, --offset, --format",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-user-topup",
    "description": "P0: 实现 user topup 命令",
    "steps": [
      "在 user.rs 添加 cmd_user_topup 函数",
      "读取 --user-id (required), --amount (required), --currency (required) 参数",
      "验证 currency 为 USD 或 CNY",
      "调用 burncloud_database_user::DbUser::update_balance 更新余额",
      "调用 burncloud_database_user::DbUser::create_recharge 创建充值记录",
      "输出充值成功信息、充值金额、新余额",
      "在 commands.rs 的 user 子命令下添加 topup 子命令",
      "topup 子命令参数: --user-id, --amount, --currency",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-user-recharges",
    "description": "P0: 实现 user recharges 命令",
    "steps": [
      "在 user.rs 添加 cmd_user_recharges 函数",
      "读取 --user-id (required), --limit (默认100) 参数",
      "调用 burncloud_database_user::DbUser::get_recharges 查询充值历史",
      "table 格式: 输出 ID, Amount, Currency, Description, CreatedAt",
      "在 commands.rs 的 user 子命令下添加 recharges 子命令",
      "recharges 子命令参数: --user-id, --limit",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-user-check-username",
    "description": "P0: 实现 user check-username 命令",
    "steps": [
      "在 user.rs 添加 cmd_user_check_username 函数",
      "读取 --username (required) 参数",
      "调用数据库查询用户名是否存在",
      "输出 Available 或 Already taken",
      "在 commands.rs 的 user 子命令下添加 check-username 子命令",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-channel-update",
    "description": "P1: 实现 channel update 命令",
    "steps": [
      "在 crates/cli/src/channel.rs 添加 cmd_channel_update 函数",
      "读取位置参数 <id> (required)",
      "读取可选参数: --name, --key, --status, --models, --priority, --weight, --base-url, --pricing-region",
      "调用 ChannelModel::get_by_id 验证渠道存在，不存在则报错",
      "构建更新后的 Channel 结构体，仅更新提供的字段",
      "调用 ChannelModel::update 保存更新",
      "输出更新成功信息",
      "在 commands.rs 的 channel 子命令下添加 update 子命令",
      "update 子命令: 位置参数 id, 可选参数 --name, --key, --status, --models, --priority, --weight, --base-url, --pricing-region",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-group-create",
    "description": "P1: 实现 group create 命令",
    "steps": [
      "新建文件 crates/cli/src/group.rs",
      "添加模块导入和 cmd_group_create 函数",
      "读取 --name (required), --members (optional, 逗号分隔) 参数",
      "调用数据库创建路由组",
      "如有 members 参数，同时添加组成员",
      "输出创建成功的组 ID",
      "实现 handle_group_command 路由函数",
      "在 lib.rs 添加 mod group;",
      "在 commands.rs 添加 Command::new(\"group\") 和 create 子命令",
      "create 子命令参数: --name (required), --members (optional)",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-group-list",
    "description": "P1: 实现 group list 命令",
    "steps": [
      "在 group.rs 添加 cmd_group_list 函数",
      "读取 --format (默认table) 参数",
      "调用数据库查询所有路由组",
      "table 格式: 输出 ID, Name, MemberCount, CreatedAt",
      "json 格式: 输出 JSON 数组",
      "在 commands.rs 的 group 子命令下添加 list 子命令",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-group-show",
    "description": "P1: 实现 group show 命令",
    "steps": [
      "在 group.rs 添加 cmd_group_show 函数",
      "读取位置参数 <id> (required)",
      "调用数据库查询组详情",
      "输出: ID, Name, CreatedAt",
      "输出组成员列表 (member_id, upstream_id, weight)",
      "在 commands.rs 的 group 子命令下添加 show 子命令",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-group-delete",
    "description": "P1: 实现 group delete 命令",
    "steps": [
      "在 group.rs 添加 cmd_group_delete 函数",
      "读取位置参数 <id> (required) 和 -y/--yes 确认标志",
      "无 -y 时显示确认提示: Delete group 'name' (ID: x)? [y/N]",
      "用户输入 y/yes 才执行删除",
      "调用数据库删除组和关联的成员记录",
      "输出删除成功信息",
      "在 commands.rs 的 group 子命令下添加 delete 子命令",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-group-members",
    "description": "P1: 实现 group members 命令",
    "steps": [
      "在 group.rs 添加 cmd_group_members 函数",
      "读取位置参数 <id> (required) 和可选 --set 参数",
      "无 --set 时: 查询并显示当前成员列表",
      "有 --set 时: 解析逗号分隔的成员列表 (格式: upstream_id:weight 或 upstream_id)",
      "先删除现有成员，再插入新成员",
      "输出操作成功信息",
      "在 commands.rs 的 group 子命令下添加 members 子命令",
      "members 子命令: 位置参数 id, 可选参数 --set",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-log-list",
    "description": "P2: 实现 log list 命令",
    "steps": [
      "新建文件 crates/cli/src/log.rs",
      "添加模块导入和 cmd_log_list 函数",
      "读取 --user-id (optional), --channel-id (optional), --model (optional), --limit (默认100), --offset (默认0) 参数",
      "调用数据库查询请求日志 (logs 表)",
      "table 格式: 输出 ID, UserID, ChannelID, Model, PromptTokens, CompletionTokens, Cost, Timestamp",
      "json 格式: 输出 JSON 数组",
      "实现 handle_log_command 路由函数",
      "在 lib.rs 添加 mod log;",
      "在 commands.rs 添加 Command::new(\"log\") 和 list 子命令",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-log-usage",
    "description": "P2: 实现 log usage 命令",
    "steps": [
      "在 log.rs 添加 cmd_log_usage 函数",
      "读取 --user-id (required), --period (day/week/month, 默认month) 参数",
      "聚合查询指定时间段内的使用数据",
      "输出: 总请求数、总 Prompt Tokens、总 Completion Tokens、总费用 (USD/CNY)",
      "可选: 按模型分组的统计",
      "在 commands.rs 的 log 子命令下添加 usage 子命令",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-monitor-status",
    "description": "P2: 实现 monitor status 命令",
    "steps": [
      "新建文件 crates/cli/src/monitor.rs",
      "添加模块导入和 cmd_monitor_status 函数",
      "读取 --format (默认table) 参数",
      "查询系统指标: 总渠道数、活跃渠道数、今日请求数、今日 Token 数、今日收入",
      "table 格式: 输出指标名称和数值",
      "json 格式: 输出 JSON 对象",
      "实现 handle_monitor_command 路由函数",
      "在 lib.rs 添加 mod monitor;",
      "在 commands.rs 添加 Command::new(\"monitor\") 和 status 子命令",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "cli-test-user",
    "description": "黑盒测试: user 命令",
    "steps": [
      "编译 release: cargo build --release -p burncloud",
      "测试 user register: ./target/release/burncloud user register --username test_cli_user --password test123",
      "测试 user list: ./target/release/burncloud user list",
      "测试 user check-username: ./target/release/burncloud user check-username --username test_cli_user",
      "测试 user topup: ./target/release/burncloud user topup --user-id 1 --amount 10.5 --currency USD",
      "测试 user recharges: ./target/release/burncloud user recharges --user-id 1",
      "验证所有命令输出正确，无报错",
      "清理测试数据（如需要）"
    ],
    "passes": true
  },
  {
    "category": "cli-test-channel-group",
    "description": "黑盒测试: channel/group 命令",
    "steps": [
      "测试 channel update: ./target/release/burncloud channel update 1 --status 2 --priority 10",
      "测试 channel update --pricing-region: ./target/release/burncloud channel update 1 --pricing-region cn",
      "测试 group create: ./target/release/burncloud group create --name test-group",
      "测试 group list: ./target/release/burncloud group list",
      "测试 group members: ./target/release/burncloud group members 1",
      "测试 group members --set: ./target/release/burncloud group members 1 --set 1:100,2:50",
      "测试 group show: ./target/release/burncloud group show 1",
      "测试 group delete: ./target/release/burncloud group delete 1 -y",
      "验证所有命令输出正确，无报错"
    ],
    "passes": true
  },
  {
    "category": "cli-test-log-monitor",
    "description": "黑盒测试: log/monitor 命令",
    "steps": [
      "测试 log list: ./target/release/burncloud log list --limit 10",
      "测试 log list --user-id: ./target/release/burncloud log list --user-id 1 --limit 5",
      "测试 log usage: ./target/release/burncloud log usage --user-id 1",
      "测试 monitor status: ./target/release/burncloud monitor status",
      "测试 JSON 格式: ./target/release/burncloud log list --format json --limit 3",
      "验证所有命令输出正确，无报错"
    ],
    "passes": true
  },
  {
    "category": "price-get-region-commands",
    "description": "P0-1: commands.rs 添加 price get --region 参数",
    "steps": [
      "打开 crates/cli/src/commands.rs",
      "找到 Command::new(\"get\") 子命令定义（约第259行）",
      "在现有参数后添加: .arg(Arg::new(\"region\").long(\"region\").help(\"Filter by region (cn, international)\"))",
      "位置: 在 Arg::new(\"verbose\") 之前",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "price-get-region-cli",
    "description": "P0-2: price.rs (CLI) 修改 price get 处理逻辑",
    "steps": [
      "打开 crates/cli/src/price.rs",
      "找到 Some((\"get\", sub_m)) 分支（约第144行）",
      "在 let currency = ... 之后添加: let region = sub_m.get_one::<String>(\"region\").map(|s| s.as_str());",
      "修改第152行: PriceModel::get(db, model, curr, None) 改为 PriceModel::get(db, model, curr, region)",
      "修改第190行: PriceModel::get_all_currencies(db, model, None) 改为 PriceModel::get_all_currencies(db, model, region)",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "price-get-region-test",
    "description": "P0-3: 测试 price get --region 功能",
    "steps": [
      "编译: cargo build --release -p burncloud",
      "准备测试数据: ./target/release/burncloud price set test-get-region --input 1.0 --output 2.0 --region cn",
      "测试带 region 查询: ./target/release/burncloud price get test-get-region --currency USD --region cn",
      "验证: 输出应显示 Model, Currency, Input Price, Output Price, Region: cn",
      "测试无 region 查询: ./target/release/burncloud price get test-get-region --currency USD",
      "验证: 应返回空或回退到 universal 价格",
      "清理: ./target/release/burncloud price delete test-get-region"
    ],
    "passes": true
  },
  {
    "category": "price-list-region-commands",
    "description": "P0-4: commands.rs 添加 price list --region 参数",
    "steps": [
      "打开 crates/cli/src/commands.rs",
      "找到 Command::new(\"list\") 子命令定义（在 price 子命令下，约第181行）",
      "在 Arg::new(\"currency\") 之后添加: .arg(Arg::new(\"region\").long(\"region\").help(\"Filter by region (cn, international)\"))",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "price-list-region-db",
    "description": "P0-5: database price.rs 修改 list 函数签名",
    "steps": [
      "打开 crates/database/crates/database-models/src/price.rs",
      "找到 pub async fn list 函数（约第248行）",
      "修改函数签名，添加参数: region: Option<&str>",
      "完整签名: pub async fn list(db: &Database, limit: i32, offset: i32, currency: Option<&str>, region: Option<&str>) -> Result<Vec<Price>>"
    ],
    "passes": true
  },
  {
    "category": "price-list-region-sql",
    "description": "P0-6: database price.rs 修改 list 函数 SQL 查询",
    "steps": [
      "在 list 函数中，根据 region 参数构建 SQL WHERE 条件",
      "当 region.is_some() 时，在 SQL 中添加 AND region IS NOT DISTINCT FROM ? 条件",
      "PostgreSQL 使用 IS NOT DISTINCT FROM，SQLite 使用 (region = ? OR (region IS NULL AND ? IS NULL))",
      "在 sqlx::query_as 调用中 bind region 参数",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "price-list-region-cli",
    "description": "P0-7: price.rs (CLI) 修改 price list 处理逻辑",
    "steps": [
      "打开 crates/cli/src/price.rs",
      "找到 Some((\"list\", sub_m)) 分支（约第29行）",
      "在 let currency = ... 之后添加: let region = sub_m.get_one::<String>(\"region\").map(|s| s.as_str());",
      "修改 PriceModel::list 调用，传递 region 参数",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "price-list-region-test",
    "description": "P0-8: 测试 price list --region 功能",
    "steps": [
      "编译: cargo build --release -p burncloud",
      "测试 cn 区域过滤: ./target/release/burncloud price list --region cn",
      "验证: 列表仅显示 region=cn 的模型（deepseek-chat, qwen-max 等）",
      "测试 international 区域过滤: ./target/release/burncloud price list --region international",
      "验证: 列表仅显示 region=international 的模型",
      "测试无区域过滤: ./target/release/burncloud price list",
      "验证: 显示所有价格"
    ],
    "passes": true
  },
  {
    "category": "price-delete-region-commands",
    "description": "P1-1: commands.rs 添加 price delete --region 参数",
    "steps": [
      "打开 crates/cli/src/commands.rs",
      "找到 Command::new(\"delete\") 子命令定义（在 price 子命令下）",
      "在 Arg::new(\"model\") 之后添加: .arg(Arg::new(\"region\").long(\"region\").help(\"Delete only for a specific region\"))",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "price-delete-region-db",
    "description": "P1-2: database price.rs 添加 delete_by_region 函数",
    "steps": [
      "打开 crates/database/crates/database-models/src/price.rs",
      "在 impl PriceModel 块中添加新函数",
      "函数签名: pub async fn delete_by_region(db: &Database, model: &str, region: &str) -> Result<u64>",
      "实现 SQL: DELETE FROM prices WHERE model = ? AND region = ?",
      "同时支持 PostgreSQL ($1, $2) 和 SQLite (?, ?) 语法",
      "返回删除的行数"
    ],
    "passes": true
  },
  {
    "category": "price-delete-region-cli",
    "description": "P1-3: price.rs (CLI) 修改 price delete 处理逻辑",
    "steps": [
      "打开 crates/cli/src/price.rs",
      "找到 Some((\"delete\", sub_m)) 分支（约第138行）",
      "添加读取 region 参数: let region = sub_m.get_one::<String>(\"region\").map(|s| s.as_str());",
      "修改逻辑: if let Some(r) = region { PriceModel::delete_by_region(db, model, r).await?; } else { PriceModel::delete_all_for_model(db, model).await?; }",
      "更新输出信息: 带region时输出 'Deleted {region} region price for {model}'，不带时输出 'All prices deleted'",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "price-delete-region-test",
    "description": "P1-4: 测试 price delete --region 功能",
    "steps": [
      "编译: cargo build --release -p burncloud",
      "准备测试数据: ./target/release/burncloud price set test-del-region --input 1.0 --output 2.0 --region cn",
      "准备测试数据: ./target/release/burncloud price set test-del-region --input 0.8 --output 1.5 --region international",
      "删除 cn 区域: ./target/release/burncloud price delete test-del-region --region cn",
      "验证 cn 已删除: ./target/release/burncloud price get test-del-region --region cn（应为空）",
      "验证 international 保留: ./target/release/burncloud price get test-del-region --region international（应显示）",
      "清理: ./target/release/burncloud price delete test-del-region"
    ],
    "passes": true
  },
  {
    "category": "price-set-priority-commands",
    "description": "P2-1: commands.rs 添加 price set --priority-input/output 参数",
    "steps": [
      "打开 crates/cli/src/commands.rs",
      "找到 price set 子命令的 Command::new(\"set\") 定义",
      "在现有参数后添加两个新参数:",
      "  .arg(Arg::new(\"priority-input\").long(\"priority-input\").help(\"Priority input price per 1M tokens\"))",
      "  .arg(Arg::new(\"priority-output\").long(\"priority-output\").help(\"Priority output price per 1M tokens\"))",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "price-set-priority-cli",
    "description": "P2-2: price.rs (CLI) 解析 priority 参数",
    "steps": [
      "打开 crates/cli/src/price.rs",
      "找到 Some((\"set\", sub_m)) 分支",
      "在 batch_output_price 解析后添加:",
      "  let priority_input_price: Option<f64> = sub_m.get_one::<String>(\"priority-input\").and_then(|s| s.parse().ok());",
      "  let priority_output_price: Option<f64> = sub_m.get_one::<String>(\"priority-output\").and_then(|s| s.parse().ok());",
      "在 PriceInput 构建中设置: priority_input_price: priority_input_price.map(to_nano), priority_output_price: priority_output_price.map(to_nano),",
      "在输出中添加: if let Some(pi) = priority_input_price { println!(\"  Priority input: {:.4}/1M\", pi); }",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "price-set-audio-commands",
    "description": "P2-3: commands.rs 添加 price set --audio-input 参数",
    "steps": [
      "打开 crates/cli/src/commands.rs",
      "在 price set 子命令的 priority-output 参数后添加:",
      "  .arg(Arg::new(\"audio-input\").long(\"audio-input\").help(\"Audio input price per 1M tokens\"))",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "price-set-audio-cli",
    "description": "P2-4: price.rs (CLI) 解析 audio 参数",
    "steps": [
      "打开 crates/cli/src/price.rs",
      "在 priority_output_price 解析后添加:",
      "  let audio_input_price: Option<f64> = sub_m.get_one::<String>(\"audio-input\").and_then(|s| s.parse().ok());",
      "在 PriceInput 构建中设置: audio_input_price: audio_input_price.map(to_nano),",
      "在输出中添加: if let Some(ai) = audio_input_price { println!(\"  Audio input: {:.4}/1M\", ai); }",
      "编译验证: cargo build -p burncloud"
    ],
    "passes": true
  },
  {
    "category": "price-set-advanced-test",
    "description": "P2-5: 测试 price set 高级定价参数",
    "steps": [
      "编译: cargo build --release -p burncloud",
      "测试 priority 参数: ./target/release/burncloud price set gpt-4o-test --input 2.5 --output 10.0 --priority-input 4.25 --priority-output 17.0",
      "验证输出包含: Priority input: 4.2500/1M, Priority output: 17.0000/1M",
      "测试 audio 参数: ./target/release/burncloud price set gpt-4o-audio --input 2.5 --output 10.0 --audio-input 17.5",
      "验证输出包含: Audio input: 17.5000/1M",
      "验证数据库: ./target/release/burncloud price get gpt-4o-test -v",
      "清理: ./target/release/burncloud price delete gpt-4o-test && ./target/release/burncloud price delete gpt-4o-audio"
    ],
    "passes": true
  },
  {
    "category": "price-region-integration-test",
    "description": "集成测试: price 模块所有 region 功能",
    "steps": [
      "编译: cargo build --release -p burncloud",
      "创建多区域价格: ./target/release/burncloud price set test-integration --input 1.0 --output 2.0 --region cn --currency CNY",
      "创建多区域价格: ./target/release/burncloud price set test-integration --input 0.15 --output 0.3 --region international --currency USD",
      "测试 list 过滤: ./target/release/burncloud price list --region cn | grep test-integration",
      "测试 get cn: ./target/release/burncloud price get test-integration --currency CNY --region cn",
      "测试 get international: ./target/release/burncloud price get test-integration --currency USD --region international",
      "删除 cn: ./target/release/burncloud price delete test-integration --region cn",
      "验证 cn 已删除 international 保留: ./target/release/burncloud price list --region international | grep test-integration",
      "清理: ./target/release/burncloud price delete test-integration"
    ],
    "passes": true
  }
]
