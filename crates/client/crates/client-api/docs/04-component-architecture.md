# 组件架构文档

## 架构概述

BurnCloud Client API 采用现代化的组件架构设计，基于 Dioxus 框架构建。整个应用程序采用声明式 UI 模式，通过组件化的方式组织代码结构，确保高可维护性和可扩展性。

## 核心架构模式

### 1. 组件化架构
- **单一职责**: 每个组件专注于特定功能
- **组合模式**: 通过组件组合构建复杂界面
- **状态管理**: 集中式状态管理和局部状态
- **事件驱动**: 基于事件的交互模式

### 2. 层次结构

```
App (根组件)
├── ApiManagement (API 管理主组件)
│   ├── PageHeader (页面头部)
│   └── PageContent (页面内容)
│       └── ApiCard (API 卡片)
│           ├── ApiEndpoint (端点信息)
│           └── StatusIndicator (状态指示器)
```

## 组件详细说明

### 1. App 根组件

**文件位置**: `src/main.rs:14-22`

```rust
#[component]
fn App() -> Element {
    rsx! {
        // style { {include_str!("../assets/styles.css")} }
        div { id: "app",
            ApiManagement {}
        }
    }
}
```

**职责说明**:
- 应用程序的顶级容器
- 提供全局样式加载（当前已注释）
- 渲染主要的 API 管理组件

**设计特点**:
- 简洁的结构，专注于组件组合
- 提供应用级别的容器 div
- 样式系统的入口点

### 2. ApiManagement 主组件

**文件位置**: `src/api.rs:4-45`

```rust
#[component]
pub fn ApiManagement() -> Element {
    rsx! {
        div { class: "page-header",
            h1 { class: "text-large-title font-bold text-primary m-0",
                "API管理"
            }
            p { class: "text-secondary m-0 mt-sm",
                "管理和配置API接口"
            }
        }

        div { class: "page-content",
            div { class: "card",
                div { class: "p-lg",
                    h3 { class: "text-subtitle font-semibold mb-md", "API端点" }
                    div { class: "flex flex-col gap-md",
                        // API 端点列表
                    }
                }
            }
        }
    }
}
```

**职责说明**:
- 应用程序的主要功能组件
- 管理 API 端点的显示和状态
- 提供用户界面的主要布局结构

**组件结构**:

#### 2.1 PageHeader (页面头部)
```rust
div { class: "page-header",
    h1 { class: "text-large-title font-bold text-primary m-0",
        "API管理"
    }
    p { class: "text-secondary m-0 mt-sm",
        "管理和配置API接口"
    }
}
```
- **功能**: 显示页面标题和描述信息
- **样式类**: 使用标准化的文本和间距类
- **内容**: 固定的标题文本

#### 2.2 PageContent (页面内容)
```rust
div { class: "page-content",
    div { class: "card",
        div { class: "p-lg",
            h3 { class: "text-subtitle font-semibold mb-md", "API端点" }
            div { class: "flex flex-col gap-md",
                // API 端点项目
            }
        }
    }
}
```
- **功能**: 主要内容区域容器
- **布局**: 卡片式设计，包含内边距
- **结构**: 垂直布局，间距统一

### 3. ApiEndpoint 端点组件

**当前实现** (内联在 ApiManagement 中):
```rust
div { class: "flex justify-between items-center p-md border-b",
    div {
        div { class: "font-medium", "/v1/chat/completions" }
        div { class: "text-caption text-secondary", "对话完成接口" }
    }
    span { class: "status-indicator status-running",
        span { class: "status-dot" }
        "正常"
    }
}
```

**组件职责**:
- 显示单个 API 端点信息
- 展示端点路径和描述
- 集成状态指示器

**数据结构**:
```rust
// 建议的数据模型
struct ApiEndpoint {
    path: String,
    description: String,
    status: EndpointStatus,
    method: HttpMethod,
}

enum EndpointStatus {
    Running,
    Stopped,
    Error,
    Maintenance,
}

enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
}
```

### 4. StatusIndicator 状态组件

**当前实现**:
```rust
span { class: "status-indicator status-running",
    span { class: "status-dot" }
    "正常"
}
```

**组件职责**:
- 可视化显示服务状态
- 提供颜色编码的状态指示
- 支持不同状态类型

**状态类型映射**:
- `status-running`: 🟢 正常运行（绿色）
- `status-stopped`: 🔴 已停止（红色）
- `status-warning`: 🟡 警告（黄色）
- `status-maintenance`: 🔵 维护中（蓝色）

## 数据流架构

### 1. 组件通信模式

```
User Interaction
       ↓
Event Handler
       ↓
State Update
       ↓
Component Re-render
       ↓
UI Update
```

### 2. 状态管理

**当前状态管理**:
- 静态数据展示
- 无动态状态更新
- 组件内部状态管理

**建议的状态管理结构**:
```rust
use dioxus::prelude::*;

// 全局状态
#[derive(Clone, PartialEq)]
struct AppState {
    api_endpoints: Vec<ApiEndpoint>,
    connection_status: ConnectionStatus,
    current_view: ViewMode,
}

// 使用 Context 进行状态管理
fn use_app_state() -> &UseRef<AppState> {
    use_context::<UseRef<AppState>>()
}
```

## 样式架构

### 1. CSS 类系统

**原子化类设计**:
```css
/* 文本类 */
.text-large-title  { font-size: 2.5rem; }
.text-subtitle     { font-size: 1.25rem; }
.text-caption      { font-size: 0.875rem; }

/* 颜色类 */
.text-primary      { color: #1a73e8; }
.text-secondary    { color: #666; }

/* 布局类 */
.flex             { display: flex; }
.flex-col         { flex-direction: column; }
.justify-between  { justify-content: space-between; }
.items-center     { align-items: center; }

/* 间距类 */
.m-0              { margin: 0; }
.p-md             { padding: 1rem; }
.gap-md           { gap: 1rem; }
```

### 2. 组件样式映射

| 组件 | 主要样式类 | 用途 |
|------|------------|------|
| PageHeader | `page-header` | 页面头部容器 |
| Card | `card` | 卡片容器样式 |
| StatusIndicator | `status-indicator` | 状态指示器 |
| ApiEndpoint | `border-b` | 端点项分隔线 |

## 扩展架构

### 1. 组件扩展模式

**新增组件步骤**:
1. 在 `src/components/` 创建组件文件
2. 实现 `#[component]` 函数
3. 导出组件到 `lib.rs`
4. 在父组件中引用

**示例扩展**:
```rust
// src/components/api_metrics.rs
#[component]
pub fn ApiMetrics() -> Element {
    rsx! {
        div { class: "metrics-card",
            h3 { "API 统计" }
            // 统计数据显示
        }
    }
}
```

### 2. 状态扩展

**建议的状态扩展**:
```rust
// 添加异步状态管理
use dioxus::prelude::*;

#[component]
pub fn ApiManagement() -> Element {
    let endpoints = use_resource(|| async {
        fetch_api_endpoints().await
    });

    match endpoints.read().as_ref() {
        Some(Ok(data)) => rsx! {
            // 渲染端点列表
        },
        Some(Err(_)) => rsx! { div { "加载失败" } },
        None => rsx! { div { "加载中..." } },
    }
}
```

## 性能优化

### 1. 组件优化
- 使用 `memo` 缓存不变的组件
- 避免不必要的重新渲染
- 合理使用 `key` 属性

### 2. 状态优化
- 局部化状态管理
- 避免深层状态嵌套
- 使用 `use_selector` 精确订阅

## 测试架构

### 1. 组件测试结构
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_testing::*;

    #[test]
    fn test_api_management_render() {
        let mut dom = VirtualDom::new(ApiManagement);
        let _ = dom.rebuild();

        // 测试组件渲染
        assert!(dom.base_scope().has_context::<AppState>());
    }
}
```

### 2. 集成测试
- 端到端组件交互测试
- 状态变更测试
- 用户交互流程测试

---

*本文档描述了 BurnCloud Client API 的完整组件架构，为后续开发提供了清晰的结构指导。*