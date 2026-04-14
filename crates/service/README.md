# burncloud-service

Service 层聚合器。通过 `pub use` 重导出 12 个业务逻辑子 crate,提供统一访问入口。

## 为什么存在

避免上层代码为每个 Service 子 crate 添加依赖。`use burncloud_service::token::TokenService` 一行搞定。

## 子 Crate 一览

| 子 Crate | 职责 |
|----------|------|
| service-billing | 多模态计费(PriceCache, CostCalculator) |
| service-inference | 本地推理进程管理 |
| service-models | 模型元数据 + HuggingFace 集成 |
| service-monitor | 系统监控(CPU/内存/磁盘) |
| service-user | 用户注册/登录/JWT |
| service-token | API Token 管理 + 配额 |
| service-router-log | 路由日志 + 账单统计 |
| service-group | 分组管理 |
| service-upstream | 上游管理 |
| service-setting | KV 系统配置 |
| service-redis | Redis 连接池和操作 |
| service-ip | IP 地理位置(CN/WORLD) |

## 使用方式

```rust
use burncloud_service::token::TokenService;
use burncloud_service::billing::PriceCache;
use burncloud_service::monitor::SystemMonitorService;
```

## 依赖

所有 `burncloud-service-*` 子 crate。

## Sources

- `crates/service/src/lib.rs` — 聚合器
- `docs/backend/service-patterns.md` — 开发指南
