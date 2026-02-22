# burncloud-client

Dioxus-based GUI 客户端，支持 LiveView Web 模式。

## 架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           burncloud-client                                   │
│                          (GUI / LiveView)                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                           Router (Dioxus)                              │  │
│  │                                                                        │  │
│  │   /             ──►  home.rs        # 首页                            │  │
│  │   /login        ──►  login.rs       # 登录                            │  │
│  │   /dashboard    ──►  dashboard.rs   # 仪表盘                          │  │
│  │   /models       ──►  models.rs      # 模型管理                        │  │
│  │   /monitor      ──►  monitor.rs     # 系统监控                        │  │
│  │   /logs         ──►  logs.rs        # 日志查询                        │  │
│  │   /settings     ──►  settings.rs    # 设置                            │  │
│  │   /billing      ──►  billing.rs     # 账单                            │  │
│  │   /playground   ──►  playground.rs  # API 测试                        │  │
│  │   /api          ──►  api.rs         # API 管理                        │  │
│  │   /connect      ──►  connect.rs     # 连接设置                        │  │
│  │   /deploy       ──►  deploy.rs      # 部署                            │  │
│  │                                                                        │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                      │                                       │
│                                      ▼                                       │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                         components/                                    │  │
│  │  ┌────────────────┐  ┌────────────────┐                              │  │
│  │  │    layout.rs   │  │  guest_layout  │                              │  │
│  │  │                │  │     .rs        │                              │  │
│  │  │  主布局组件    │  │  访客布局      │                              │  │
│  │  └────────────────┘  └────────────────┘                              │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                          子 Crates                                     │  │
│  │                                                                        │  │
│  │  client-dashboard  client-monitor  client-users  client-finance       │  │
│  │  client-access     client-log      client-connect client-playground   │  │
│  │  client-register   client-settings client-models  client-deploy       │  │
│  │  client-shared     client-tray     client-api                         │  │
│  │                                                                        │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 模块清单

| 模块 | 文件 | 职责 |
|------|------|------|
| **lib.rs** | `lib.rs` | 入口，`liveview_router()` |
| **app.rs** | `app.rs` | Dioxus `App` 组件 |
| **main.rs** | `main.rs` | GUI 启动入口 |

### pages/ - 页面组件

| 页面 | 文件 | 职责 |
|------|------|------|
| home | `pages/home.rs` | 首页 |
| login | `pages/login.rs` | 登录页 |
| dashboard | `pages/dashboard.rs` | 仪表盘 |
| models | `pages/models.rs` | 模型管理 |
| monitor | `pages/monitor.rs` | 系统监控 |
| logs | `pages/logs.rs` | 日志查询 |
| settings | `pages/settings.rs` | 设置 |
| billing | `pages/billing.rs` | 账单 |
| playground | `pages/playground.rs` | API 测试 |
| api | `pages/api.rs` | API 管理 |
| connect | `pages/connect.rs` | 连接设置 |
| deploy | `pages/deploy.rs` | 部署 |
| user | `pages/user.rs` | 用户管理 |
| not_found | `pages/not_found.rs` | 404 页面 |

### components/ - 可复用组件

| 组件 | 文件 | 职责 |
|------|------|------|
| layout | `components/layout.rs` | 主布局 |
| guest_layout | `components/guest_layout.rs` | 访客布局 |

## 子 Crates

| Crate | 职责 |
|-------|------|
| client-dashboard | 仪表盘功能 |
| client-monitor | 监控功能 |
| client-users | 用户管理功能 |
| client-finance | 财务功能 |
| client-access | 访问控制 |
| client-log | 日志功能 |
| client-connect | 连接功能 |
| client-playground | API 测试功能 |
| client-register | 注册功能 |
| client-settings | 设置功能 |
| client-models | 模型功能 |
| client-deploy | 部署功能 |
| client-shared | 共享组件 |
| client-tray | 系统托盘 |
| client-api | API 客户端 |

## 运行模式

```
cargo run            # GUI 模式 (Windows)
cargo run -- server  # Server + LiveView 模式 (Linux)
```

## 关键函数

```rust
// LiveView 路由器
pub fn liveview_router(db: Arc<Database>) -> Router;

// GUI 启动
pub fn launch_gui();
```

## 依赖关系

```
burncloud-client
├── burncloud-database    # 数据库访问
├── burncloud-service-*   # 服务层
├── 子 crates
│   ├── client-dashboard
│   ├── client-monitor
│   └── ...
└── external: dioxus, dioxus-liveview
```

## 设计特点

- **Dioxus 框架** - Rust 原生 UI 框架
- **LiveView 支持** - 服务端渲染，无需前端构建
- **Windows 11 Fluent Design** - 现代化 UI 风格
- **i18n** - 支持中英文
