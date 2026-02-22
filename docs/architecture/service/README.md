# burncloud-service

业务逻辑层，纯业务代码，无 UI 依赖。

## 架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           burncloud-service                                  │
│                           (Business Logic Layer)                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐                │
│  │service-inference│  │ service-user  │  │service-models  │                │
│  │                │  │                │  │                │                │
│  │ 推理服务逻辑   │  │ 用户管理服务  │  │ 模型管理服务   │                │
│  └────────────────┘  └────────────────┘  └────────────────┘                │
│                                                                              │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐                │
│  │service-monitor │  │service-setting │  │ service-redis  │                │
│  │                │  │                │  │                │                │
│  │ 系统监控服务   │  │ 设置服务      │  │ Redis 缓存服务 │                │
│  │                │  │                │  │                │                │
│  │ ┌────────────┐ │  │                │  │                │                │
│  │ │ collectors │ │  │                │  │                │                │
│  │ │ ├── cpu    │ │  │                │  │                │                │
│  │ │ ├── memory │ │  │                │  │                │                │
│  │ │ └── disk   │ │  │                │  │                │                │
│  │ └────────────┘ │  │                │  │                │                │
│  └────────────────┘  └────────────────┘  └────────────────┘                │
│                                                                              │
│  ┌────────────────┐                                                         │
│  │  service-ip    │                                                         │
│  │                │                                                         │
│  │ IP 地理位置    │                                                         │
│  └────────────────┘                                                         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 子 Crate 清单

| Crate | 目录 | 职责 |
|-------|------|------|
| **service-inference** | `crates/service-inference/` | 推理服务逻辑 |
| **service-user** | `crates/service-user/` | 用户管理服务 |
| **service-models** | `crates/service-models/` | 模型管理服务 |
| **service-monitor** | `crates/service-monitor/` | 系统监控服务 |
| **service-setting** | `crates/service-setting/` | 设置服务 |
| **service-redis** | `crates/service-redis/` | Redis 缓存服务 |
| **service-ip** | `crates/service-ip/` | IP 地理位置 |

## service-monitor 详解

```
service-monitor
├── lib.rs              # 入口
├── service.rs          # SystemMonitorService
├── types.rs            # 监控类型定义
└── collectors/         # 数据采集器
    ├── mod.rs
    ├── cpu.rs          # CPU 使用率
    ├── memory.rs       # 内存使用
    └── disk.rs         # 磁盘使用
```

### SystemMonitorService

```rust
pub struct SystemMonitorService {
    // 系统监控数据
}

impl SystemMonitorService {
    pub fn new() -> Self;
    pub async fn start_auto_update(&self) -> Result<()>;
    // ... 采集方法
}
```

## 依赖关系

```
burncloud-service
├── service-inference
├── service-user
├── service-models
├── service-monitor
│   └── collectors (cpu, memory, disk)
├── service-setting
├── service-redis
└── service-ip
```

## 设计原则

- **纯业务逻辑** - 不依赖 UI 框架
- **可测试性** - 独立的业务逻辑便于单元测试
- **可复用** - 可被 server、client、cli 等多处调用
