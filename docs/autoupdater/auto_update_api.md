# 自动更新模块 API 文档

## 概述

自动更新模块提供从 GitHub 和 Gitee 自动更新应用程序的功能，支持回退机制。当 GitHub 更新失败时，会自动尝试从 Gitee 更新。

## 核心结构

### UpdateConfig

更新配置结构体，包含仓库信息和版本信息。

```rust
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    pub github_owner: String,    // GitHub 仓库所有者
    pub github_repo: String,     // GitHub 仓库名称
    pub gitee_owner: String,     // Gitee 仓库所有者
    pub gitee_repo: String,      // Gitee 仓库名称
    pub bin_name: String,        // 二进制文件名
    pub current_version: String, // 当前版本
}
```

#### 默认配置

```rust
impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            github_owner: "burncloud".to_string(),
            github_repo: "burncloud".to_string(),
            gitee_owner: "burncloud".to_string(),
            gitee_repo: "burncloud".to_string(),
            bin_name: "burncloud".to_string(),
            current_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
```

### AutoUpdater

自动更新器结构体，提供更新检查和执行功能。

```rust
pub struct AutoUpdater {
    config: UpdateConfig,
}
```

## API 方法

### 构造方法

#### `new(config: UpdateConfig) -> Self`

使用指定配置创建自动更新器。

**参数:**
- `config`: 更新配置

**返回值:**
- `AutoUpdater` 实例

**示例:**
```rust
let config = UpdateConfig {
    github_owner: "myorg".to_string(),
    github_repo: "myapp".to_string(),
    // ... 其他配置
    ..Default::default()
};
let updater = AutoUpdater::new(config);
```

#### `with_default_config() -> Self`

使用默认配置创建自动更新器。

**返回值:**
- `AutoUpdater` 实例

**示例:**
```rust
let updater = AutoUpdater::with_default_config();
```

### 核心功能

#### `check_for_updates(&self) -> Result<bool>`

检查是否有可用更新。先尝试 GitHub，失败则尝试 Gitee。

**返回值:**
- `Ok(true)`: 有可用更新
- `Ok(false)`: 已是最新版本
- `Err(e)`: 检查失败

**示例:**
```rust
match updater.check_for_updates().await {
    Ok(true) => println!("发现新版本！"),
    Ok(false) => println!("已是最新版本"),
    Err(e) => eprintln!("检查更新失败: {}", e),
}
```

#### `update_with_fallback(&self) -> Result<()>`

执行更新，带回退机制。先尝试从 GitHub 更新，失败则尝试从 Gitee 更新。

**返回值:**
- `Ok(())`: 更新成功
- `Err(e)`: 更新失败

**示例:**
```rust
match updater.update_with_fallback().await {
    Ok(_) => println!("更新成功！"),
    Err(e) => eprintln!("更新失败: {}", e),
}
```

### 辅助方法

#### `current_version(&self) -> &str`

获取当前版本号。

**返回值:**
- 当前版本字符串

#### `set_config(&mut self, config: UpdateConfig)`

设置新的配置。

**参数:**
- `config`: 新的更新配置

## 私有方法

### GitHub 相关

#### `check_github_updates(&self) -> Result<bool>`

从 GitHub 检查更新。

#### `update_from_github(&self) -> Result<()>`

从 GitHub 执行更新。

### Gitee 相关

#### `check_gitee_updates(&self) -> Result<bool>`

从 Gitee 检查更新（使用 S3 后端模拟）。

#### `update_from_gitee(&self) -> Result<()>`

从 Gitee 执行更新。

## 使用示例

### 基本使用

```rust
use burncloud_common::{AutoUpdater, UpdateConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    env_logger::init();

    // 创建更新器
    let updater = AutoUpdater::with_default_config();

    // 检查更新
    if updater.check_for_updates().await? {
        println!("发现新版本，开始更新...");

        // 执行更新
        updater.update_with_fallback().await?;
        println!("更新完成！");
    } else {
        println!("已是最新版本");
    }

    Ok(())
}
```

### 自定义配置

```rust
use burncloud_common::{AutoUpdater, UpdateConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = UpdateConfig {
        github_owner: "mycompany".to_string(),
        github_repo: "myproject".to_string(),
        gitee_owner: "mycompany".to_string(),
        gitee_repo: "myproject".to_string(),
        bin_name: "myapp".to_string(),
        current_version: "1.0.0".to_string(),
    };

    let updater = AutoUpdater::new(config);

    match updater.update_with_fallback().await {
        Ok(_) => println!("✅ 更新成功"),
        Err(e) => eprintln!("❌ 更新失败: {}", e),
    }

    Ok(())
}
```

### 仅检查更新

```rust
let updater = AutoUpdater::with_default_config();

if updater.check_for_updates().await? {
    println!("有新版本可用，请运行更新命令");
} else {
    println!("当前已是最新版本");
}
```

## 错误处理

模块使用 `anyhow::Result` 进行错误处理，主要错误类型包括：

- 网络连接错误
- 仓库不存在或无法访问
- 版本解析错误
- 文件下载或替换失败

## 日志记录

模块使用 `log` crate 记录重要事件：

- `info!`: 正常操作信息
- `warn!`: 警告信息（如 GitHub 失败回退到 Gitee）
- `error!`: 错误信息

确保在使用前初始化日志记录器：

```rust
env_logger::init();
```

## 依赖要求

- `self_update = "0.40"`
- `anyhow = "1.0"`
- `log = "0.4"`
- `tokio` (用于异步操作)

## 注意事项

1. 需要网络连接才能检查和下载更新
2. 应用程序需要有写入权限才能替换自身
3. 更新后建议重启应用程序
4. 确保 GitHub 和 Gitee 仓库中有相应的 Release 版本