# burncloud-database-setting

BurnCloud 设置管理数据库库,提供简单的键值对存储功能。

## 功能特性

- ✅ 基于 `burncloud-database` 构建
- ✅ 简洁的键值对存储
- ✅ 支持增加、修改、删除、查询操作
- ✅ 主键唯一性保证
- ✅ 异步 API 设计

## 数据库表结构

```sql
CREATE TABLE IF NOT EXISTS setting (
    name TEXT PRIMARY KEY,    -- 设置项名称(主键)
    value TEXT NOT NULL       -- 设置项值
)
```

## 安装

在 `Cargo.toml` 中添加依赖:

```toml
[dependencies]
burncloud-database-setting.workspace = true
```

## 使用示例

```rust
use burncloud_database_setting::SettingDatabase;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建数据库实例
    let db = SettingDatabase::new().await?;

    // 添加或更新设置项
    db.set("theme", "dark").await?;
    db.set("language", "zh-CN").await?;

    // 获取设置值
    if let Some(theme) = db.get("theme").await? {
        println!("当前主题: {}", theme);
    }

    // 列出所有设置
    let settings = db.list_all().await?;
    for setting in settings {
        println!("{} = {}", setting.name, setting.value);
    }

    // 删除设置项
    db.delete("theme").await?;

    // 关闭数据库
    db.close().await?;

    Ok(())
}
```

## API 文档

### `SettingDatabase::new()`

创建新的设置数据库实例,自动初始化数据表。

```rust
let db = SettingDatabase::new().await?;
```

### `set(name: &str, value: &str)`

添加或更新设置项。如果 name 已存在,则更新其值。

```rust
db.set("app_name", "BurnCloud").await?;
```

### `get(name: &str) -> Option<String>`

根据 name 获取设置值。返回 `Some(value)` 如果存在,否则返回 `None`。

```rust
if let Some(value) = db.get("app_name").await? {
    println!("应用名称: {}", value);
}
```

### `delete(name: &str)`

删除指定的设置项。

```rust
db.delete("old_setting").await?;
```

### `list_all() -> Vec<Setting>`

获取所有设置项列表,按 name 排序。

```rust
let settings = db.list_all().await?;
for setting in settings {
    println!("{} = {}", setting.name, setting.value);
}
```

### `close()`

关闭数据库连接。

```rust
db.close().await?;
```

## 运行示例

```bash
# 运行使用示例
cargo run -p burncloud-database-setting --example usage
```

## 项目结构

```
crates/database/crates/database-setting/
├── Cargo.toml           # 项目配置
├── README.md            # 说明文档
├── src/
│   └── lib.rs          # 核心实现
└── examples/
    └── usage.rs        # 使用示例
```

## 依赖项

- `burncloud-database` - 核心数据库抽象层
- `sqlx` - SQL 数据库工具包
- `tokio` - 异步运行时
- `serde` / `serde_json` - 序列化支持

## 许可证

MIT OR Apache-2.0
