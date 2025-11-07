# burncloud-service-setting

BurnCloud 设置服务层，提供简洁的配置管理接口。

## 功能特性

- ✅ 基于 `burncloud-database-setting` 构建
- ✅ 简洁的服务层 API
- ✅ 支持增加、修改、删除、查询操作
- ✅ 异步接口设计
- ✅ 自动处理数据库连接

## 架构说明

此服务层封装了 `burncloud-database-setting`，提供更高层次的业务逻辑接口。服务层与数据层分离，便于：
- 业务逻辑的集中管理
- API 的统一封装
- 未来扩展（如缓存、权限控制等）

## 安装

在 `Cargo.toml` 中添加依赖:

```toml
[dependencies]
burncloud-service-setting.workspace = true
```

## 使用示例

```rust
use burncloud_service_setting::SettingService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建服务实例
    let service = SettingService::new().await?;

    // 设置配置项
    service.set("theme", "dark").await?;
    service.set("language", "zh-CN").await?;

    // 获取配置值
    if let Some(theme) = service.get("theme").await? {
        println!("当前主题: {}", theme);
    }

    // 列出所有配置
    let settings = service.list_all().await?;
    for setting in settings {
        println!("{} = {}", setting.name, setting.value);
    }

    // 删除配置项
    service.delete("theme").await?;

    // 关闭服务
    service.close().await?;

    Ok(())
}
```

## API 文档

### `SettingService::new()`

创建新的设置服务实例，自动初始化数据库。

```rust
let service = SettingService::new().await?;
```

### `set(name: &str, value: &str)`

设置配置项。如果 name 已存在，则更新其值；否则新增。

```rust
service.set("app_name", "BurnCloud").await?;
```

### `get(name: &str) -> Option<String>`

根据 name 获取配置值。返回 `Some(value)` 如果存在，否则返回 `None`。

```rust
if let Some(value) = service.get("app_name").await? {
    println!("应用名称: {}", value);
}
```

### `delete(name: &str)`

删除指定的配置项。

```rust
service.delete("old_setting").await?;
```

### `list_all() -> Vec<Setting>`

获取所有配置项列表。

```rust
let settings = service.list_all().await?;
for setting in settings {
    println!("{} = {}", setting.name, setting.value);
}
```

### `close()`

关闭服务，释放数据库连接。

```rust
service.close().await?;
```

## 运行示例

```bash
# 运行使用示例
cargo run -p burncloud-service-setting --example usage
```

## 项目结构

```
crates/service/crates/service-setting/
├── Cargo.toml           # 项目配置
├── README.md            # 说明文档
├── src/
│   └── lib.rs          # 服务层实现
└── examples/
    └── usage.rs        # 使用示例
```

## 依赖项

- `burncloud-database-setting` - 设置数据库层
- `burncloud-database` - 核心数据库抽象层
- `tokio` - 异步运行时

## 与其他 crate 的关系

```
burncloud-service-setting (服务层)
    └── burncloud-database-setting (数据库层)
            └── burncloud-database (核心抽象层)
                    └── sqlx (SQLite ORM)
```

## 许可证

MIT OR Apache-2.0
