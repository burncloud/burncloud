# burncloud-cli

命令行工具，提供管理操作接口。

## 架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            burncloud-cli                                     │
│                         (Command Line Tool)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                          commands.rs                                   │  │
│  │                          (命令路由)                                    │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                      │                                       │
│            ┌─────────────────────────┼─────────────────────────┐            │
│            │                         │                         │            │
│            ▼                         ▼                         ▼            │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────────┐  │
│  │    price.rs      │    │   currency.rs    │    │    channel.rs        │  │
│  │                  │    │                  │    │                      │  │
│  │ price list       │    │ currency list    │    │ channel list         │  │
│  │ price set        │    │ currency get     │    │ channel add          │  │
│  │ price get        │    │ currency set     │    │ channel update       │  │
│  │ price delete     │    │                  │    │ channel delete       │  │
│  └──────────────────┘    └──────────────────┘    └──────────────────────┘  │
│                                                                              │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────────┐  │
│  │    token.rs      │    │   protocol.rs    │    │    client.rs         │  │
│  │                  │    │                  │    │                      │  │
│  │ token list       │    │ protocol list    │    │ API 客户端           │  │
│  │ token create     │    │ protocol get     │    │                      │  │
│  │ token delete     │    │ protocol set     │    │                      │  │
│  └──────────────────┘    └──────────────────┘    └──────────────────────┘  │
│                                                                              │
│  ┌──────────────────┐                                                       │
│  │    output.rs     │                                                       │
│  │                  │                                                       │
│  │ 输出格式化       │                                                       │
│  └──────────────────┘                                                       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 模块清单

| 模块 | 文件 | 职责 |
|------|------|------|
| **lib.rs** | `lib.rs` | 入口 |
| **commands** | `commands.rs` | 命令路由和分发 |
| **price** | `price.rs` | 价格管理命令 |
| **currency** | `currency.rs` | 汇率管理命令 |
| **channel** | `channel.rs` | 通道管理命令 |
| **token** | `token.rs` | Token 管理命令 |
| **protocol** | `protocol.rs` | 协议配置命令 |
| **client** | `client.rs` | API 客户端 |
| **output** | `output.rs` | 输出格式化 |

## CLI 命令

### 价格管理

```bash
# 列出所有模型价格
burncloud price list

# 设置模型价格
burncloud price set <model> --input <price> --output <price>

# 获取模型价格详情
burncloud price get <model>

# 删除模型价格
burncloud price delete <model>
```

### 分段定价管理

```bash
# 列出分段定价
burncloud tiered list-tiers <model> [--region <cn|international>]

# 添加分段定价
burncloud tiered add-tier <model> --tier-start <tokens> --tier-end <tokens> \
    --input-price <price> --output-price <price> [--region <region>]

# 导入分段定价
burncloud tiered import-tiered <file.json>

# 检查是否有分段定价
burncloud tiered check-tiered <model>

# 删除分段定价
burncloud tiered delete-tiers <model> [--region <region>]
```

### 汇率管理

```bash
# 列出汇率
burncloud currency list

# 获取汇率
burncloud currency get <from> <to>

# 设置汇率
burncloud currency set <from> <to> <rate>
```

### 通道管理

```bash
burncloud channel list
burncloud channel add
burncloud channel update
burncloud channel delete
```

### Token 管理

```bash
burncloud token list
burncloud token create
burncloud token delete
```

### 协议配置

```bash
burncloud protocol list
burncloud protocol get <channel_type>
burncloud protocol set <channel_type> <config.json>
```

## 关键结构

```rust
pub struct Cli {
    // CLI 配置
}

impl Cli {
    pub async fn run(&self) -> anyhow::Result<()>;
}
```

## 依赖关系

```
burncloud-cli
├── burncloud-database      # 数据库访问
│   ├── database-models     # 价格模型
│   └── database-router     # 路由数据
├── burncloud-common        # 共享类型
└── external: clap, tokio
```
