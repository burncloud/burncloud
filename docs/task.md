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
  }
]
