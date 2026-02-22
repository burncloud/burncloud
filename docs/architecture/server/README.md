# burncloud-server

控制平面 (Control Plane)，提供 REST API 和 LiveView UI。

## 架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           burncloud-server                                   │
│                            (Control Plane)                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                          Router (axum)                                 │  │
│  │                                                                        │  │
│  │   /api/*  ──────────────────────────►  api::routes()                  │  │
│  │   /*       ──────────────────────────►  LiveView or Router fallback   │  │
│  │                                                                        │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                      │                                       │
│            ┌─────────────────────────┼─────────────────────────┐            │
│            │                         │                         │            │
│            ▼                         ▼                         ▼            │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────────┐  │
│  │    api/auth.rs   │    │   api/*.rs       │    │  LiveView            │  │
│  │  ┌────────────┐  │    │  ┌────────────┐  │    │  (Dioxus)            │  │
│  │  │auth_       │  │    │  │ user.rs    │  │    │                      │  │
│  │  │middleware  │  │    │  │ channel.rs │  │    │  burncloud_client::  │  │
│  │  │Claims      │  │    │  │ token.rs   │  │    │  liveview_router()   │  │
│  │  └────────────┘  │    │  │ group.rs   │  │    │                      │  │
│  └──────────────────┘    │  │ log.rs     │  │    └──────────────────────┘  │
│                          │  │ monitor.rs │  │                               │
│                          │  └────────────┘  │                               │
│                          └──────────────────┘                               │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                          AppState                                      │  │
│  │  ├── db: Arc<Database>                                                │  │
│  │  └── monitor: Arc<SystemMonitorService>                               │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 模块清单

| 模块 | 文件 | 职责 |
|------|------|------|
| **lib.rs** | `lib.rs` | 主入口：`start_server()`，`create_app()` |
| **api/auth** | `api/auth.rs` | JWT 认证中间件，Claims |
| **api/user** | `api/user.rs` | 用户管理 API |
| **api/channel** | `api/channel.rs` | 通道管理 API |
| **api/token** | `api/token.rs` | Token 管理 API |
| **api/group** | `api/group.rs` | 分组管理 API |
| **api/log** | `api/log.rs` | 日志查询 API |
| **api/monitor** | `api/monitor.rs` | 系统监控 API |

## API 路由

```
/api/*
├── /auth/*        # 认证相关
├── /users/*       # 用户管理
├── /channels/*    # 通道管理
├── /tokens/*      # Token 管理
├── /groups/*      # 分组管理
├── /logs/*        # 日志查询
└── /monitor/*     # 系统监控
```

## 关键函数

```rust
// 启动服务器
pub async fn start_server(port: u16, enable_liveview: bool) -> anyhow::Result<()>

// 创建应用
pub async fn create_app(db: Arc<Database>, enable_liveview: bool) -> anyhow::Result<Router>
```

## 启动流程

```
start_server()
    │
    ├── create_default_database()
    │
    ├── RouterDatabase::init()
    │
    ├── UserDatabase::init()
    │
    ├── start_price_sync_task()  ──► 后台价格同步 (每小时)
    │
    ├── create_app()
    │   │
    │   ├── SystemMonitorService::new()
    │   │
    │   ├── api::routes()         ──► 管理 API
    │   │
    │   ├── liveview_router()     ──► LiveView UI (可选)
    │   │
    │   └── create_router_app()   ──► 数据平面 (fallback)
    │
    └── axum::serve()
```

## 依赖关系

```
burncloud-server
├── burncloud-database      # 数据库
│   ├── database-router     # 路由数据
│   └── database-user       # 用户数据
├── burncloud-router        # 数据平面
├── burncloud-client        # LiveView UI
├── burncloud-service-monitor  # 系统监控
└── external: axum, tower-http
```
