# 自动更新架构说明

## 当前实现

已成功实现了基于 `self_update` crate 的自动更新功能，包括：

### 功能特性

1. **自动更新模块** (`crates/common/src/auto_update.rs`)
   - 基于 GitHub Releases 的自动更新
   - 简洁的配置管理
   - 完整的错误处理
   - 手动下载链接回退

2. **CLI 集成** (`crates/cli/src/commands.rs`)
   - `burncloud update` - 执行更新
   - `burncloud update --check-only` - 仅检查更新
   - 用户友好的进度提示
   - 详细的错误信息

3. **日志记录** (`src/main.rs`)
   - 使用 `env_logger` 初始化
   - 支持 `RUST_LOG` 环境变量控制

4. **完整文档** (`doc/`)
   - API 文档 (`auto_update_api.md`)
   - 设计文档 (`auto_update_design.md`)
   - 使用示例 (`auto_update_examples.md`)

### 技术栈

```toml
[dependencies]
self_update = "0.40"
anyhow = "1.0"
log = "0.4"
env_logger = "0.11"
```

## 当前已知问题

### 1. Tokio 运行时冲突

在测试过程中发现 `self_update` crate 与现有的 Tokio 运行时存在冲突：

```
Cannot drop a runtime in a context where blocking is not allowed.
This happens when a runtime is dropped from within an asynchronous context.
```

### 原因分析

1. `self_update` crate 内部创建了自己的运行时
2. 我们的应用程序已经在 `#[tokio::main]` 异步上下文中
3. 嵌套运行时导致冲突

### 解决方案

#### 方案 1: 使用 `spawn_blocking`

```rust
pub async fn update_with_fallback(&self) -> Result<()> {
    let config = self.config.clone();
    tokio::task::spawn_blocking(move || {
        // 在阻塞线程中执行更新逻辑
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            // 更新逻辑
        })
    }).await??;
    Ok(())
}
```

#### 方案 2: 分离同步/异步接口

```rust
impl AutoUpdater {
    // 同步版本
    pub fn sync_update(&self) -> Result<()> {
        // 使用阻塞 API
    }

    // 异步版本（用于其他上下文）
    pub async fn async_update(&self) -> Result<()> {
        // 异步实现
    }
}
```

#### 方案 3: 替代 crate

考虑使用其他自动更新库：
- `self-replace` + 手动 HTTP 请求
- `update-cli` (如果存在)
- 自制解决方案

## 推荐实现方案

### 短期解决方案

使用 `spawn_blocking` 包装 self_update 调用：

```rust
pub async fn update_with_fallback(&self) -> Result<()> {
    let config = self.config.clone();

    tokio::task::spawn_blocking(move || {
        // 创建同步版本的更新器
        let update = github::Update::configure()
            .repo_owner(&config.github_owner)
            .repo_name(&config.github_repo)
            .bin_name(&config.bin_name)
            .current_version(&config.current_version)
            .build()?;

        update.update()
    }).await??;

    Ok(())
}
```

### 长期解决方案

自实现更新逻辑：

1. **获取 Release 信息**
   ```rust
   let url = format!("https://api.github.com/repos/{}/{}/releases/latest", owner, repo);
   let response: Release = reqwest::get(&url).await?.json().await?;
   ```

2. **下载二进制文件**
   ```rust
   let asset = response.assets.iter()
       .find(|a| a.name.contains(&target_platform()))?;
   let bytes = reqwest::get(&asset.download_url).await?.bytes().await?;
   ```

3. **替换可执行文件**
   ```rust
   self_replace::self_replace(&bytes)?;
   ```

## 项目结构

```
crates/common/src/
├── auto_update.rs          # 自动更新核心模块
├── lib.rs                  # 模块导出

crates/cli/src/
├── commands.rs             # CLI 命令处理 (包含 update 命令)

doc/
├── auto_update_api.md      # API 文档
├── auto_update_design.md   # 设计文档
├── auto_update_examples.md # 使用示例
└── architecture.md         # 本文档

Cargo.toml                  # 根依赖配置
```

## 测试策略

### 单元测试

```bash
cargo test -p burncloud-common auto_update
```

### 集成测试

```bash
# 检查更新 (当前有运行时问题)
cargo run -- update --check-only

# 执行更新 (当前有运行时问题)
cargo run -- update
```

### 模拟测试

可以创建一个模拟 GitHub API 的测试环境。

## 部署考虑

### CI/CD 集成

1. **GitHub Actions** 自动构建 Release
2. **语义版本控制** 确保版本一致性
3. **多平台构建** Windows, Linux, macOS

### Release 准备

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ['v*']
jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v2
      - name: Build
        run: cargo build --release
      - name: Upload artifacts
        # 上传到 GitHub Releases
```

## 安全性

1. **HTTPS 强制**: 所有网络请求使用 HTTPS
2. **签名验证**: self_update crate 内置支持
3. **权限检查**: 更新前验证写入权限
4. **备份机制**: 可考虑在更新前备份当前版本

## 监控和度量

### 建议添加的指标

1. **更新成功率**
2. **更新检查频率**
3. **不同版本的使用分布**
4. **更新失败原因统计**

### 日志记录

```rust
// 当前已实现
info!("检查更新中...");
info!("正在从 GitHub 下载更新...");
error!("GitHub 更新失败: {}", e);

// 可考虑添加
metrics::counter!("auto_update.check", 1);
metrics::counter!("auto_update.success", 1);
metrics::counter!("auto_update.failure", 1);
```

## 结论

自动更新功能的基础架构已经完成，包括：

✅ **核心模块** - 完整实现
✅ **CLI 集成** - 完整实现
✅ **文档** - 详细完整
✅ **测试** - 基础测试覆盖
⚠️ **运行时兼容性** - 需要解决 Tokio 冲突

下一步需要解决运行时冲突问题，之后就可以进行实际的部署和测试了。

## 使用指南

### 当前可用功能

```bash
# 查看帮助 (运行正常)
cargo run -- --help

# 检查更新 (有运行时问题)
RUST_LOG=info cargo run -- update --check-only

# 执行更新 (有运行时问题)
cargo run -- update
```

### 解决运行时问题后

```bash
# 正常使用
burncloud update --check-only
burncloud update
```