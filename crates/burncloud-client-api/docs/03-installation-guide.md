# 安装与设置指南

## 系统要求

### 操作系统支持
- **Windows**: Windows 10/11 (64位)
- **macOS**: macOS 10.15 或更高版本
- **Linux**: Ubuntu 20.04 LTS 或其他主流发行版

### 硬件要求
- **处理器**: 支持 x86-64 架构
- **内存**: 最少 4GB RAM，推荐 8GB 或更多
- **存储**: 至少 500MB 可用磁盘空间
- **网络**: 稳定的互联网连接

## 开发环境准备

### 1. 安装 Rust 开发环境

#### Windows 系统

1. 访问 [rustup.rs](https://rustup.rs/) 下载 Rust 安装程序
2. 运行安装程序并按照提示完成安装
3. 打开新的命令提示符验证安装：

```cmd
rustc --version
cargo --version
```

#### macOS/Linux 系统

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 重新加载环境变量
source ~/.cargo/env

# 验证安装
rustc --version
cargo --version
```

### 2. 安装系统依赖

#### Windows 系统
无需额外安装，Dioxus Desktop 会自动处理 WebView2 依赖。

#### macOS 系统
```bash
# 安装 Xcode Command Line Tools（如果尚未安装）
xcode-select --install
```

#### Ubuntu/Debian 系统
```bash
# 安装必需的系统依赖
sudo apt update
sudo apt install -y webkit2gtk-4.0-dev build-essential curl wget libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

#### Arch Linux 系统
```bash
# 安装系统依赖
sudo pacman -S webkit2gtk base-devel curl wget openssl gtk3 libappindicator-gtk3 librsvg
```

#### Fedora 系统
```bash
# 安装系统依赖
sudo dnf install webkit2gtk3-devel.x86_64 openssl-devel curl wget libappindicator-gtk3-devel librsvg2-devel
sudo dnf group install "C Development Tools and Libraries"
```

## 项目安装

### 方式一：从源码构建（推荐开发者）

```bash
# 克隆仓库
git clone https://github.com/burncloud/burncloud-client-api.git
cd burncloud-client-api

# 构建项目
cargo build --release

# 运行应用程序
cargo run --release
```

### 方式二：使用预编译二进制文件

1. 访问项目的 [Releases 页面](https://github.com/burncloud/burncloud-client-api/releases)
2. 下载适合您操作系统的最新版本
3. 解压文件到目标目录
4. 运行可执行文件

#### Windows
```cmd
# 解压后运行
burncloud-client-api.exe
```

#### macOS/Linux
```bash
# 添加执行权限并运行
chmod +x burncloud-client-api
./burncloud-client-api
```

## 项目配置

### 开发环境配置

#### 1. 配置开发工具

**推荐 IDE/编辑器:**
- **VS Code** + Rust Analyzer 插件
- **IntelliJ IDEA** + Rust 插件
- **Vim/Neovim** + rust.vim

**VS Code 推荐插件:**
```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "serayuzgur.crates",
    "vadimcn.vscode-lldb"
  ]
}
```

#### 2. 代码格式化配置

创建 `rustfmt.toml` 配置文件（可选）：
```toml
# 代码格式化配置
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
```

#### 3. 代码检查配置

创建 `clippy.toml` 配置文件（可选）：
```toml
# Clippy 检查配置
avoid-breaking-exported-api = false
msrv = "1.70"
```

### 构建配置

#### 开发构建
```bash
# 普通构建（包含调试信息）
cargo build

# 运行项目
cargo run
```

#### 生产构建
```bash
# 优化构建（发布版本）
cargo build --release

# 运行优化版本
cargo run --release
```

#### 自定义构建目标
```bash
# 为特定平台构建
cargo build --target x86_64-pc-windows-gnu    # Windows
cargo build --target x86_64-apple-darwin      # macOS Intel
cargo build --target aarch64-apple-darwin     # macOS Apple Silicon
cargo build --target x86_64-unknown-linux-gnu # Linux
```

## 项目结构说明

```
burncloud-client-api/
├── src/                    # 源代码目录
│   ├── main.rs            # 应用程序入口
│   ├── lib.rs             # 库模块导出
│   └── api.rs             # API 管理组件
├── assets/                # 资源文件
│   └── styles.css         # 样式文件
├── docs/                  # 项目文档
│   ├── 01-project-overview.md
│   ├── 02-api-endpoints.md
│   └── 03-installation-guide.md
├── target/                # 构建输出目录
├── Cargo.toml            # 项目配置文件
├── Cargo.lock            # 依赖锁定文件
└── README.md             # 项目说明
```

## 验证安装

### 运行测试
```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test --lib
cargo test --bin burncloud-client-api
```

### 检查代码质量
```bash
# 运行 Clippy 检查
cargo clippy

# 格式化代码
cargo fmt

# 检查代码而不构建
cargo check
```

### 启动应用程序

成功安装后，运行以下命令启动应用：
```bash
cargo run
```

您应该看到一个标题为 "BurnCloud API管理" 的桌面窗口，显示 API 端点管理界面。

## 常见问题排查

### 构建失败

**问题**: `error: linker 'cc' not found`
**解决方案**:
```bash
# Ubuntu/Debian
sudo apt install build-essential

# macOS
xcode-select --install
```

**问题**: `error: Microsoft Visual C++ 14.0 is required` (Windows)
**解决方案**: 安装 Visual Studio Build Tools 或完整版 Visual Studio

### 运行时错误

**问题**: `WebView2 not found` (Windows)
**解决方案**: WebView2 会自动安装，如果仍有问题，手动下载安装 [Microsoft Edge WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

**问题**: 应用无法启动
**解决方案**:
1. 检查是否有其他实例在运行
2. 确认系统满足最低要求
3. 查看终端错误输出

## 更新说明

### 更新项目依赖
```bash
# 更新所有依赖到兼容版本
cargo update

# 检查可用的新版本
cargo outdated  # 需要安装 cargo-outdated
```

### 升级应用程序
```bash
# 拉取最新代码
git pull origin main

# 重新构建
cargo build --release
```

---

*安装完成后，请参考其他文档了解项目架构和开发工作流程。*