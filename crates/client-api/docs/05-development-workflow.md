# 开发工作流程文档

## 开发环境设置

### 1. 初始环境配置

```bash
# 克隆项目
git clone https://github.com/burncloud/burncloud-client-api.git
cd burncloud-client-api

# 安装开发依赖
cargo install cargo-watch cargo-edit cargo-outdated
```

### 2. IDE 配置

#### VS Code 工作区设置
创建 `.vscode/settings.json`:
```json
{
  "rust-analyzer.cargo.buildScripts.enable": true,
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.procMacro.enable": true,
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true
  }
}
```

创建 `.vscode/extensions.json`:
```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "serayuzgur.crates",
    "vadimcn.vscode-lldb",
    "tamasfe.even-better-toml"
  ]
}
```

## 开发流程

### 1. 分支管理策略

#### 主要分支
- `main`: 主分支，包含生产就绪代码
- `develop`: 开发分支，包含最新开发功能
- `feature/*`: 功能分支，开发新功能
- `bugfix/*`: 修复分支，修复 bug
- `hotfix/*`: 热修复分支，紧急修复

#### 分支命名规范
```bash
# 功能开发
feature/api-endpoint-management
feature/user-authentication

# Bug 修复
bugfix/api-status-display
bugfix/memory-leak-fix

# 热修复
hotfix/critical-security-patch
```

### 2. 功能开发工作流

#### 步骤 1: 创建功能分支
```bash
git checkout develop
git pull origin develop
git checkout -b feature/new-feature-name
```

#### 步骤 2: 开发环境运行
```bash
# 开发模式运行（支持热重载）
cargo watch -x 'run'

# 或者手动运行
cargo run
```

#### 步骤 3: 代码开发
- 编写功能代码
- 编写单元测试
- 更新文档

#### 步骤 4: 代码质量检查
```bash
# 格式化代码
cargo fmt

# 静态分析检查
cargo clippy --all-targets --all-features -- -D warnings

# 运行测试
cargo test

# 检查代码编译
cargo check
```

#### 步骤 5: 提交代码
```bash
# 添加文件
git add .

# 提交（遵循提交信息规范）
git commit -m "feat: add new API endpoint management feature"

# 推送分支
git push origin feature/new-feature-name
```

### 3. 提交信息规范

#### 提交类型
- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档变更
- `style`: 代码格式化
- `refactor`: 重构代码
- `test`: 添加测试
- `chore`: 构建过程或辅助工具变动

#### 提交信息格式
```
<type>(<scope>): <subject>

<body>

<footer>
```

#### 示例
```bash
git commit -m "feat(api): add endpoint status monitoring

- Add real-time status checking for API endpoints
- Implement status indicator with color coding
- Add automatic refresh mechanism

Closes #123"
```

## 代码规范

### 1. Rust 代码规范

#### 命名约定
```rust
// 函数和变量：snake_case
fn calculate_api_response_time() -> u64 {}
let response_time = 150;

// 类型：PascalCase
struct ApiEndpoint {}
enum EndpointStatus {}

// 常量：SCREAMING_SNAKE_CASE
const MAX_RETRY_COUNT: u32 = 3;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// 模块：snake_case
mod api_management;
mod status_indicator;
```

#### 代码组织
```rust
// 导入顺序
use std::collections::HashMap;           // 标准库
use dioxus::prelude::*;                 // 外部 crate
use crate::components::StatusIndicator; // 本地模块

#[component]
pub fn ApiManagement() -> Element {
    // 状态声明在顶部
    let endpoints = use_state(Vec::new);

    // 副作用处理
    use_effect(move |_| async move {
        // 异步逻辑
    });

    // 事件处理函数
    let handle_refresh = move |_| {
        // 处理逻辑
    };

    // 渲染 JSX
    rsx! {
        // 组件内容
    }
}
```

### 2. 组件开发规范

#### 组件文件结构
```rust
// src/components/api_endpoint.rs
use dioxus::prelude::*;

#[derive(Props, PartialEq)]
pub struct ApiEndpointProps {
    pub path: String,
    pub description: String,
    pub status: EndpointStatus,
    #[props(default)]
    pub on_click: Option<EventHandler<MouseEvent>>,
}

#[component]
pub fn ApiEndpoint(props: ApiEndpointProps) -> Element {
    rsx! {
        div {
            class: "api-endpoint",
            onclick: move |evt| {
                if let Some(handler) = &props.on_click {
                    handler.call(evt);
                }
            },

            div { class: "endpoint-info",
                div { class: "endpoint-path", "{props.path}" }
                div { class: "endpoint-desc", "{props.description}" }
            }

            StatusIndicator { status: props.status }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_testing::*;

    #[test]
    fn test_api_endpoint_render() {
        // 测试代码
    }
}
```

## 测试策略

### 1. 单元测试

#### 组件测试
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_testing::prelude::*;

    #[test]
    fn api_management_renders_correctly() {
        let mut dom = VirtualDom::new(ApiManagement);
        let _ = dom.rebuild();

        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("API管理"));
        assert!(html.contains("API端点"));
    }

    #[test]
    fn status_indicator_shows_correct_status() {
        let mut dom = VirtualDom::new(|| rsx! {
            StatusIndicator { status: EndpointStatus::Running }
        });

        let _ = dom.rebuild();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("status-running"));
    }
}
```

#### 逻辑测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_status_calculation() {
        let endpoint = ApiEndpoint::new("/v1/models", "Model list");
        assert_eq!(endpoint.calculate_health_score(), 100);
    }
}
```

### 2. 集成测试

创建 `tests/integration_test.rs`:
```rust
use burncloud_client_api::*;

#[tokio::test]
async fn test_full_app_workflow() {
    // 集成测试逻辑
}
```

### 3. 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_api_management

# 运行测试并显示输出
cargo test -- --nocapture

# 生成测试覆盖率报告
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## 构建与发布

### 1. 开发构建

```bash
# 开发构建
cargo build

# 检查构建（不生成二进制）
cargo check

# 监听文件变化自动构建
cargo watch -x check -x test -x run
```

### 2. 生产构建

```bash
# 发布构建
cargo build --release

# 构建优化
export RUSTFLAGS="-C target-cpu=native"
cargo build --release

# 跨平台构建
cargo install cross
cross build --target x86_64-pc-windows-gnu --release
```

### 3. 发布流程

#### 版本号管理
```bash
# 使用 cargo-edit 管理版本
cargo install cargo-edit

# 升级补丁版本（0.1.1 -> 0.1.2）
cargo set-version --bump patch

# 升级次版本（0.1.2 -> 0.2.0）
cargo set-version --bump minor

# 升级主版本（0.2.0 -> 1.0.0）
cargo set-version --bump major
```

#### 发布检查清单
- [ ] 所有测试通过
- [ ] 代码质量检查通过
- [ ] 文档已更新
- [ ] 版本号已更新
- [ ] CHANGELOG.md 已更新
- [ ] 构建成功
- [ ] 手动测试完成

## 调试与性能优化

### 1. 调试工具

#### 使用 LLDB
```json
// .vscode/launch.json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug BurnCloud API",
            "cargo": {
                "args": ["build", "--bin=burncloud-client-api"],
                "filter": {
                    "name": "burncloud-client-api",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

#### 日志调试
```rust
use log::{debug, info, warn, error};

#[component]
pub fn ApiManagement() -> Element {
    info!("Rendering ApiManagement component");

    let endpoints = use_state(|| {
        debug!("Initializing endpoints state");
        vec![]
    });

    rsx! {
        // 组件内容
    }
}
```

### 2. 性能监控

```bash
# 安装性能分析工具
cargo install cargo-flamegraph
cargo install cargo-profdata

# 生成火焰图
cargo flamegraph --bin burncloud-client-api

# 基准测试
cargo bench
```

## 持续集成配置

### GitHub Actions 工作流

创建 `.github/workflows/ci.yml`:
```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable

      - name: Format check
        run: cargo fmt -- --check

      - name: Clippy check
        run: cargo clippy -- -D warnings

      - name: Run tests
        run: cargo test

      - name: Build
        run: cargo build --release
```

## 代码审查流程

### 1. Pull Request 检查清单

#### 代码质量
- [ ] 代码遵循项目规范
- [ ] 无 clippy 警告
- [ ] 格式化正确
- [ ] 测试覆盖充分

#### 功能完整性
- [ ] 功能按需求实现
- [ ] 边界情况处理
- [ ] 错误处理完善
- [ ] 性能影响可接受

### 2. 审查标准

- **可读性**: 代码清晰易懂
- **维护性**: 结构合理，易于修改
- **测试性**: 充分的测试覆盖
- **性能**: 无明显性能问题
- **安全性**: 无安全漏洞

---

*此文档定义了 BurnCloud Client API 项目的完整开发工作流程，确保代码质量和项目的可持续发展。*