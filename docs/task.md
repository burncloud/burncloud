[
    {
        "category": "cli",
        "description": "Task 1.1: 添加 database 依赖到 CLI Cargo.toml",
        "passes": true,
        "steps": [
            "打开 crates/cli/Cargo.toml 文件",
            "在 [dependencies] 部分添加 burncloud-database.workspace = true",
            "验证: 编译通过 cargo check -p burncloud-cli"
        ]
    },
    {
        "category": "cli",
        "description": "Task 1.2: 添加 database-models 依赖到 CLI Cargo.toml",
        "passes": true,
        "steps": [
            "打开 crates/cli/Cargo.toml 文件",
            "在 [dependencies] 部分添加 burncloud-database-models.workspace = true",
            "验证: 编译通过 cargo check -p burncloud-cli"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.1: 创建 channel.rs 模块文件骨架",
        "passes": true,
        "steps": [
            "创建文件 crates/cli/src/channel.rs",
            "添加基本的模块结构和必要的 use 语句",
            "在 lib.rs 中添加 pub mod channel; 导出",
            "验证: cargo check -p burncloud-cli 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.2: 实现 parse_channel_type 函数",
        "passes": true,
        "steps": [
            "在 channel.rs 中实现 fn parse_channel_type(s: &str) -> Result<ChannelType>",
            "支持解析: openai, azure, anthropic, gemini, aws, vertexai, deepseek",
            "不支持时返回友好的错误信息",
            "验证: 单元测试覆盖所有支持的类型和错误情况"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.3: 实现 get_default_models 函数",
        "passes": true,
        "steps": [
            "在 channel.rs 中实现 fn get_default_models(channel_type: ChannelType) -> Vec<&'static str>",
            "openai -> [\"gpt-4\", \"gpt-4-turbo\", \"gpt-3.5-turbo\"]",
            "azure -> [\"gpt-4\", \"gpt-35-turbo\"]",
            "anthropic -> [\"claude-3-opus\", \"claude-3-sonnet\", \"claude-3-haiku\"]",
            "gemini -> [\"gemini-1.5-pro\", \"gemini-1.5-flash\", \"gemini-pro\"]",
            "aws -> [\"claude-3-sonnet\", \"claude-3-haiku\"]",
            "vertexai -> [\"gemini-1.5-pro\"]",
            "deepseek -> [\"deepseek-chat\", \"deepseek-coder\"]",
            "验证: 单元测试覆盖所有类型"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.4: 实现 get_default_base_url 函数",
        "passes": true,
        "steps": [
            "在 channel.rs 中实现 fn get_default_base_url(channel_type: ChannelType) -> Option<&'static str>",
            "openai -> Some(\"https://api.openai.com/v1\")",
            "azure -> None (必须用户指定)",
            "anthropic -> Some(\"https://api.anthropic.com/v1\")",
            "gemini -> Some(\"https://generativelanguage.googleapis.com/v1beta\")",
            "aws -> None (使用 AWS SDK)",
            "vertexai -> None (需要项目信息)",
            "deepseek -> Some(\"https://api.deepseek.com/v1\")",
            "验证: 单元测试覆盖所有类型"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.5: 定义 channel add 子命令参数结构",
        "passes": true,
        "steps": [
            "在 channel.rs 中定义 ChannelAddArgs 结构体或使用 clap 直接解析",
            "参数: -t, --type (必需): 渠道类型",
            "参数: -k, --key (必需): API 密钥",
            "参数: -m, --models (可选): 支持的模型列表",
            "参数: -u, --url (可选): 自定义 base URL",
            "参数: -n, --name (可选): 渠道名称",
            "验证: cargo check 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.6: 实现 cmd_channel_add 函数 - 参数验证",
        "passes": true,
        "steps": [
            "实现 fn cmd_channel_add(db: &Database, args: &ArgMatches) -> Result<()>",
            "解析 -t 参数，调用 parse_channel_type 转换",
            "解析 -k 参数，验证非空",
            "解析 -m 参数，如未提供则使用 get_default_models",
            "解析 -u 参数，如未提供且类型有默认值则使用 get_default_base_url",
            "对于 Azure 类型，验证 -u 参数必须提供",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.7: 实现 cmd_channel_add 函数 - 构建 Channel 结构体",
        "passes": true,
        "steps": [
            "创建 Channel 结构体实例",
            "设置 channel_type 字段",
            "设置 key 字段",
            "设置 models 字段 (逗号分隔的字符串)",
            "设置 base_url 字段",
            "设置 name 字段 (如未提供则使用默认命名规则如 \"Gemini Channel\")",
            "设置其他必要字段: status=1, priority=0, weight=1 等",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.8: 实现 cmd_channel_add 函数 - 数据库写入",
        "passes": true,
        "steps": [
            "调用 ChannelModel::create(db, &mut channel)",
            "处理返回的 channel id",
            "打印成功信息: \"Channel created with ID: {id}\"",
            "处理数据库错误，打印友好错误信息",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.9: 定义 channel list 子命令参数",
        "passes": true,
        "steps": [
            "定义 list 子命令参数",
            "参数: --format (可选): 输出格式，table 或 json，默认 table",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.10: 实现 cmd_channel_list 函数 - 数据查询",
        "passes": true,
        "steps": [
            "实现 fn cmd_channel_list(db: &Database, args: &ArgMatches) -> Result<()>",
            "调用 ChannelModel::list(db, limit, offset) 获取所有渠道",
            "使用合理的默认值: limit=100, offset=0",
            "处理空列表情况: 打印 \"No channels found\"",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.11: 实现 cmd_channel_list 函数 - 表格格式输出",
        "passes": true,
        "steps": [
            "当 format=table 时，使用 prettytable 或类似库输出表格",
            "列: ID, Name, Type, Status, Models, Base URL",
            "Status 显示为文本 (Active/Inactive)",
            "Type 显示为可读名称 (Gemini/AWS/Azure 等)",
            "Models 过长时截断显示",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.12: 实现 cmd_channel_list 函数 - JSON 格式输出",
        "passes": true,
        "steps": [
            "当 format=json 时，使用 serde_json 输出 JSON",
            "输出格式化的 JSON (pretty print)",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.13: 定义 channel delete 子命令参数",
        "passes": true,
        "steps": [
            "定义 delete 子命令参数",
            "参数: <ID> (位置参数，必需): 要删除的渠道 ID",
            "参数: -y, --yes (可选): 跳过确认提示",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.14: 实现 cmd_channel_delete 函数 - 查询渠道",
        "passes": true,
        "steps": [
            "实现 fn cmd_channel_delete(db: &Database, args: &ArgMatches) -> Result<()>",
            "解析 ID 参数",
            "调用 ChannelModel::get_by_id(db, id) 查询渠道",
            "渠道不存在时打印 \"Channel with ID {id} not found\" 并退出",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.15: 实现 cmd_channel_delete 函数 - 确认提示",
        "passes": true,
        "steps": [
            "如未指定 --yes 参数，显示确认提示",
            "提示格式: \"Delete channel '{name}' (ID: {id})? [y/N]\"",
            "读取用户输入",
            "只有 y/Y 才继续，其他输入则取消操作",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.16: 实现 cmd_channel_delete 函数 - 执行删除",
        "passes": true,
        "steps": [
            "调用 ChannelModel::delete(db, id)",
            "打印成功信息: \"Channel {id} deleted\"",
            "处理数据库错误，打印友好错误信息",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.17: 定义 channel show 子命令参数",
        "passes": true,
        "steps": [
            "定义 show 子命令参数",
            "参数: <ID> (位置参数，必需): 要显示的渠道 ID",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.18: 实现 cmd_channel_show 函数",
        "passes": true,
        "steps": [
            "实现 fn cmd_channel_show(db: &Database, args: &ArgMatches) -> Result<()>",
            "解析 ID 参数",
            "调用 ChannelModel::get_by_id(db, id) 查询渠道",
            "渠道不存在时打印 \"Channel with ID {id} not found\" 并退出",
            "显示渠道所有字段: ID, Name, Type, Status, Key (掩码显示), Models, Base URL 等",
            "Key 字段只显示前8个字符，其余用 * 掩码",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 2.19: 实现 handle_channel_command 路由函数",
        "passes": true,
        "steps": [
            "实现 pub async fn handle_channel_command(db: &Database, matches: &ArgMatches) -> Result<()>",
            "使用 match matches.subcommand() 路由到对应子命令",
            "Some((\"add\", sub_m)) => cmd_channel_add(db, sub_m)",
            "Some((\"list\", sub_m)) => cmd_channel_list(db, sub_m)",
            "Some((\"delete\", sub_m)) => cmd_channel_delete(db, sub_m)",
            "Some((\"show\", sub_m)) => cmd_channel_show(db, sub_m)",
            "_ => 打印帮助信息或错误",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 3.1: 在 commands.rs 定义 channel_add_command",
        "passes": true,
        "steps": [
            "创建 fn channel_add_command() -> Command",
            "设置 name(\"add\")",
            "设置 about(\"Add a new channel\")",
            "添加 -t/--type 参数，必须，长帮助文本",
            "添加 -k/--key 参数，必须",
            "添加 -m/--models 参数，可选",
            "添加 -u/--url 参数，可选",
            "添加 -n/--name 参数，可选",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 3.2: 在 commands.rs 定义 channel_list_command",
        "passes": true,
        "steps": [
            "创建 fn channel_list_command() -> Command",
            "设置 name(\"list\")",
            "设置 about(\"List all channels\")",
            "添加 --format 参数，可选，默认 \"table\"，可能值 [\"table\", \"json\"]",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 3.3: 在 commands.rs 定义 channel_delete_command",
        "passes": true,
        "steps": [
            "创建 fn channel_delete_command() -> Command",
            "设置 name(\"delete\")",
            "设置 about(\"Delete a channel\")",
            "添加 <ID> 位置参数，必须",
            "添加 -y/--yes 参数，可选，跳过确认",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 3.4: 在 commands.rs 定义 channel_show_command",
        "passes": true,
        "steps": [
            "创建 fn channel_show_command() -> Command",
            "设置 name(\"show\")",
            "设置 about(\"Show channel details\")",
            "添加 <ID> 位置参数，必须",
            "验证: 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 3.5: 在 commands.rs 添加 channel 顶级子命令",
        "passes": true,
        "steps": [
            "在 build_cli() 或相应函数中添加 channel 子命令",
            ".subcommand(Command::new(\"channel\").about(\"Manage API channels\").subcommand_required(true).subcommand(...))",
            "引入 channel_add_command, channel_list_command, channel_delete_command, channel_show_command",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 3.6: 在 commands.rs 添加 channel 命令处理逻辑",
        "passes": true,
        "steps": [
            "在主 match 块中添加 channel 分支",
            "Some((\"channel\", sub_m)) => { ... }",
            "创建数据库连接: let db = Database::new().await?;",
            "调用 handle_channel_command(&db, sub_m).await?;",
            "关闭数据库连接: db.close().await?;",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 4.1: 在 lib.rs 导出 channel 模块",
        "passes": true,
        "steps": [
            "打开 crates/cli/src/lib.rs",
            "添加 pub mod channel;",
            "添加 pub use channel::*; (如需要)",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "cli",
        "description": "Task 5.1: 添加 cli 可能需要的额外依赖",
        "passes": true,
        "steps": [
            "检查是否需要 prettytable-rs 用于表格输出",
            "检查是否需要 dialoguer 用于交互式确认",
            "如需要则添加到 Cargo.toml",
            "验证: cargo build 编译通过"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.1: 集成测试 - 完整构建",
        "passes": true,
        "steps": [
            "运行 cargo build",
            "确认无编译错误",
            "确认无编译警告 (或已知并接受)"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.2: 集成测试 - 添加 Gemini 渠道",
        "passes": false,
        "steps": [
            "运行 cargo run -- channel add -t gemini -k \"test-gemini-key\" -m \"gemini-pro\"",
            "验证命令执行成功",
            "验证输出包含新创建的渠道 ID"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.3: 集成测试 - 添加 Azure 渠道",
        "passes": false,
        "steps": [
            "运行 cargo run -- channel add -t azure -k \"test-azure-key\" -u \"https://test.openai.azure.com\" -m \"gpt-4\"",
            "验证命令执行成功",
            "验证输出包含新创建的渠道 ID"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.4: 集成测试 - 列出渠道 (表格格式)",
        "passes": false,
        "steps": [
            "运行 cargo run -- channel list",
            "验证输出为表格格式",
            "验证包含刚才添加的渠道",
            "验证列头正确显示"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.5: 集成测试 - 列出渠道 (JSON 格式)",
        "passes": false,
        "steps": [
            "运行 cargo run -- channel list --format json",
            "验证输出为有效 JSON",
            "验证 JSON 结构正确",
            "验证包含所有渠道数据"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.6: 集成测试 - 显示渠道详情",
        "passes": false,
        "steps": [
            "运行 cargo run -- channel show 1",
            "验证显示渠道的详细信息",
            "验证 Key 字段被掩码处理",
            "验证所有字段正确显示"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.7: 集成测试 - 删除渠道 (带确认)",
        "passes": false,
        "steps": [
            "运行 cargo run -- channel delete 1",
            "验证显示确认提示",
            "输入 'y' 确认",
            "验证删除成功消息",
            "运行 cargo run -- channel list 验证渠道已删除"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.8: 集成测试 - 删除渠道 (跳过确认)",
        "passes": false,
        "steps": [
            "运行 cargo run -- channel delete 2 --yes",
            "验证无需确认直接删除",
            "验证删除成功消息"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.9: 集成测试 - 删除不存在的渠道",
        "passes": false,
        "steps": [
            "运行 cargo run -- channel delete 999",
            "验证显示 \"Channel not found\" 错误消息",
            "验证命令非零退出"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.10: 集成测试 - 无效渠道类型",
        "passes": false,
        "steps": [
            "运行 cargo run -- channel add -t invalid -k \"test\"",
            "验证显示友好的错误消息",
            "验证提示支持的渠道类型"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.11: 集成测试 - 验证数据库状态",
        "passes": false,
        "steps": [
            "使用 sqlite3 命令或代码检查 ~/.burncloud/data.db",
            "验证 channels 表数据正确",
            "验证 channel_type 值正确 (数字)",
            "验证 models 字段格式正确"
        ]
    },
    {
        "category": "integration",
        "description": "Task 6.12: 集成测试 - 帮助信息",
        "passes": false,
        "steps": [
            "运行 cargo run -- channel --help",
            "验证显示所有子命令",
            "运行 cargo run -- channel add --help",
            "验证显示所有参数说明"
        ]
    }
]
