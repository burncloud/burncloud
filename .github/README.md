# GitHub 自动打包和发布配置

这个项目配置了自动化的 CI/CD 流程，可以在版本更新时自动构建和发布 Windows 版本。

## 工作流说明

### 1. CI 工作流 (`.github/workflows/ci.yml`)
- **触发条件**: 推送到主分支或创建 Pull Request
- **功能**:
  - 代码格式检查 (`cargo fmt`)
  - 代码质量检查 (`cargo clippy`)
  - 运行测试 (`cargo test`)
  - 多平台构建测试 (Ubuntu, Windows, macOS)

### 2. 版本检查工作流 (`.github/workflows/version-check.yml`)
- **触发条件**:
  - 推送到主分支且修改了 `Cargo.toml` 文件
  - 手动触发
- **功能**:
  - 检测 `Cargo.toml` 中的版本变化
  - 自动创建对应的 Git 标签
  - 触发发布流程

### 3. 发布工作流 (`.github/workflows/release.yml`)
- **触发条件**:
  - 推送新的版本标签 (格式: `v*`)
  - 手动触发 (可指定版本)
- **功能**:
  - 构建 Windows x64 版本
  - 创建可执行文件压缩包
  - 创建安装器压缩包
  - 自动生成更新日志
  - 创建 GitHub Release

## 使用方法

### 自动发布新版本

1. 修改根目录的 `Cargo.toml` 文件中的版本号：
   ```toml
   [package]
   version = "0.2.0"  # 从 0.1.0 改为 0.2.0
   ```

2. 提交并推送到主分支：
   ```bash
   git add Cargo.toml
   git commit -m "🔖 chore: bump version to 0.2.0"
   git push origin main
   ```

3. 版本检查工作流会自动：
   - 检测到版本变化
   - 创建 `v0.2.0` 标签
   - 触发发布工作流

4. 发布工作流会自动：
   - 构建 Windows 版本
   - 创建 GitHub Release
   - 上传构建产物

### 手动触发发布

在 GitHub 仓库的 Actions 页面：

1. 选择 "Release" 工作流
2. 点击 "Run workflow"
3. 输入要发布的版本号 (如: `v1.0.0`)
4. 点击 "Run workflow"

### 发布产物

每次成功发布会生成以下文件：

1. **`burncloud-vX.X.X-windows-x64.zip`**
   - 包含单个可执行文件 `burncloud.exe`
   - 适合高级用户直接使用

2. **`burncloud-vX.X.X-windows-x64-installer.zip`**
   - 包含 `burncloud.exe` 和 `install.bat`
   - 适合普通用户，双击 `install.bat` 即可安装

## 配置要求

### GitHub 仓库设置

确保仓库有以下权限：
- Actions 读写权限
- Contents 写权限 (用于创建 Release)

### 本地开发

推荐在本地进行以下检查：

```bash
# 格式检查
cargo fmt --all -- --check

# 代码质量检查
cargo clippy --all-targets --all-features -- -D warnings

# 运行测试
cargo test --all

# 构建检查
cargo build --release
```

## 故障排除

### 常见问题

1. **工作流没有触发**
   - 检查是否修改了 `Cargo.toml` 文件
   - 确认推送到了正确的分支 (`main` 或 `master`)

2. **构建失败**
   - 检查代码是否通过本地测试
   - 查看 Actions 页面的详细错误信息

3. **发布创建失败**
   - 确认标签名称格式正确 (以 `v` 开头)
   - 检查是否已存在同名 Release

### 手动操作

如需手动创建标签：

```bash
# 创建标签
git tag -a v1.0.0 -m "Release v1.0.0"

# 推送标签
git push origin v1.0.0
```

## 自定义配置

可以根据项目需要修改以下配置：

- **支持的平台**: 在 `release.yml` 中添加更多构建目标
- **发布频道**: 修改为预发布版本或测试版本
- **构建选项**: 调整 `cargo build` 参数
- **文件打包**: 修改生成的压缩包内容