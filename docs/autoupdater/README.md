# BurnCloud 自动更新功能

## 🎯 概述

基于 `self_update` crate 实现的自动更新功能，支持从 GitHub 自动检查和下载最新版本，失败时提供手动下载链接作为备选方案。

## ✨ 功能特性

- ✅ **自动检查更新** - 从 GitHub Releases 检查最新版本
- ✅ **一键更新** - 自动下载并替换应用程序
- ✅ **仅检查模式** - 只检查更新不执行更新
- ✅ **回退机制** - 失败时提供手动下载链接
- ✅ **日志记录** - 详细的操作日志
- ✅ **CLI 集成** - 完整的命令行界面
- ✅ **详细文档** - 完整的 API 和使用文档

## 🚀 快速开始

### 命令行使用

```bash
# 检查是否有更新
burncloud update --check-only

# 执行更新
burncloud update

# 查看帮助
burncloud --help
```

### 编程接口

```rust
use burncloud_common::AutoUpdater;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    env_logger::init();

    // 创建更新器
    let updater = AutoUpdater::with_default_config();

    // 检查更新
    if updater.check_for_updates().await? {
        println!("发现新版本！");

        // 执行更新
        updater.update_with_fallback().await?;
        println!("更新完成！");
    }

    Ok(())
}
```

## 📁 项目结构

```
├── packages/common/src/auto_update.rs  # 核心更新模块
├── packages/cli/src/commands.rs        # CLI 命令集成
├── src/main.rs                         # 日志初始化
├── doc/
│   ├── auto_update_api.md             # API 文档
│   ├── auto_update_design.md          # 设计文档
│   ├── auto_update_examples.md        # 使用示例
│   └── architecture.md                # 架构说明
└── Cargo.toml                          # 依赖配置
```

## 🔧 配置

### 默认配置

```rust
UpdateConfig {
    github_owner: "burncloud",
    github_repo: "burncloud",
    bin_name: "burncloud",
    current_version: env!("CARGO_PKG_VERSION"),
}
```

### 自定义配置

```rust
let config = UpdateConfig {
    github_owner: "myorg".to_string(),
    github_repo: "myapp".to_string(),
    bin_name: "myapp".to_string(),
    current_version: "1.0.0".to_string(),
};

let updater = AutoUpdater::new(config);
```

## 📖 文档

- **[API 文档](doc/auto_update_api.md)** - 详细的 API 参考
- **[设计文档](doc/auto_update_design.md)** - 模块设计思路
- **[使用示例](doc/auto_update_examples.md)** - 实际使用案例
- **[架构说明](doc/architecture.md)** - 技术架构分析

## ⚠️ 当前已知问题

### Tokio 运行时冲突

当前版本在运行时存在 Tokio 运行时冲突问题：

```
Cannot drop a runtime in a context where blocking is not allowed.
```

**临时解决方案：**
1. 在独立进程中运行更新命令
2. 使用 `spawn_blocking` 包装更新调用
3. 考虑替换为其他更新库

详见 [架构说明](doc/architecture.md#当前已知问题)。

## 🛠️ 开发

### 构建项目

```bash
# 检查代码
cargo check

# 构建项目
cargo build

# 运行测试
cargo test -p burncloud-common auto_update
```

### 启用日志

```bash
# 设置日志级别
export RUST_LOG=info

# 或在 Windows
set RUST_LOG=info

# 运行命令
cargo run -- update --check-only
```

## 📋 依赖项

```toml
self_update = "0.40"    # 核心更新功能
anyhow = "1.0"          # 错误处理
log = "0.4"             # 日志记录
env_logger = "0.11"     # 日志初始化
tokio = "1.0"           # 异步运行时
```

## 🔐 安全性

- ✅ **HTTPS 强制** - 所有网络请求使用 HTTPS
- ✅ **官方验证** - 基于 self_update crate 的内置安全机制
- ✅ **权限检查** - 更新前验证文件写入权限
- ✅ **版本校验** - 严格的版本号格式检查

## 🎯 未来计划

- [ ] 解决 Tokio 运行时冲突
- [ ] 添加增量更新支持
- [ ] 实现自动更新调度
- [ ] 支持更多更新源
- [ ] 添加回滚功能
- [ ] 集成进度条显示
- [ ] 添加更新通知机制

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

### 开发流程

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 📄 许可证

本项目使用 MIT 许可证 - 详见 LICENSE 文件。

## 🆘 获取帮助

- 查看 [文档目录](doc/) 获取详细信息
- 提交 [Issue](https://github.com/burncloud/burncloud/issues) 报告问题
- 参考 [使用示例](doc/auto_update_examples.md) 了解用法

---

## 📊 实现状态

| 功能 | 状态 | 说明 |
|------|------|------|
| 核心更新模块 | ✅ 完成 | 基于 self_update crate |
| CLI 集成 | ✅ 完成 | 支持 `--check-only` 参数 |
| 配置管理 | ✅ 完成 | 支持默认和自定义配置 |
| 错误处理 | ✅ 完成 | 完整的错误信息和回退方案 |
| 日志记录 | ✅ 完成 | 使用 log + env_logger |
| 文档 | ✅ 完成 | 4 个详细文档文件 |
| 单元测试 | ✅ 完成 | 基础测试覆盖 |
| 运行时兼容 | ⚠️ 待修复 | Tokio 冲突问题 |

**总体完成度：85%** 🎉