# 四层架构规范

> **版本**: v1.0
> **最后更新**: 2026-05-19

---

## 一、架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                    Gateway Layer (数据面)                     │
│                    crates/router                              │
│  职责: 高并发流量处理、认证、限流、协议转换、零拷贝转发          │
└─────────────────────────────────────────────────────────────┘
                              ↓ (宪法例外: service-billing, service-user)
┌─────────────────────────────────────────────────────────────┐
│                    Control Layer (控制面)                     │
│                    crates/server                              │
│  职责: RESTful API、路由注册、Handler、状态管理                │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                    Service Layer (业务逻辑)                   │
│                    crates/service                             │
│  职责: 计费、监控、用户管理、Token 管理、Channel 管理           │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                    Data Layer (数据持久化)                    │
│                    crates/database                            │
│  职责: SQL 执行、Schema、Placeholder 工具、Repository trait    │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                    Common Layer (共享工具)                    │
│                    crates/common                              │
│  职责: CrudRepository trait、跨 crate 纯工具                  │
└─────────────────────────────────────────────────────────────┘
```

---

## 二、依赖方向

### 严格单向

```
Server → Service → Database → Common
```

**禁止反向依赖**（除宪法例外）

### 依赖检查

```bash
# 检查 Service 是否依赖 Router（禁止）
grep -r "use burncloud_router" crates/service/

# 检查 Database 是否依赖 Service（禁止）
grep -r "use burncloud_service" crates/database/
```

---

## 三、宪法例外

### Router 对 Service 层的反向依赖

`burncloud-router` 作为数据面独立组件，存在以下两个对 Service 层的反向依赖例外：

| 例外依赖 | 使用的类型/方法 | 理由 |
|----------|----------------|------|
| `router → service-billing` | `PriceCache`, `CostCalculator`, `UnifiedUsage`, `BillingError`, `get_parser`, `parse_chunk_or_default`, `parse_response_or_default`, `UnifiedTokenCounter` | `PriceCache` 是带后台刷新的业务缓存，下沉到 Database 层会污染纯数据层 |
| `router → service-user` | `UserService::resolve_traffic_class` | 流量分类逻辑与 Router 数据面强耦合，下沉到 Database 层会引入权限判断语义 |

### 例外范围限制

**仅允许以上两个 `burncloud-service-*` crate。**

新增任何 `burncloud-service-*` 依赖到 `burncloud-router` 必须经过架构 review。

---

## 四、层级职责

### Gateway Layer (crates/router)

**职责**：
- 高并发流量处理
- 认证、限流
- 协议转换（OpenAI/Claude/Gemini → OpenAI 格式）
- 零拷贝转发

**关键原则**：Router 是智能管道，不处理业务逻辑

**文件结构**：
```
crates/router/
├── lib.rs              # 主路由逻辑、请求分发
├── channel_state.rs    # Channel 状态管理
├── model_router.rs     # 模型路由决策
├── passthrough.rs      # 零拷贝转发核心
├── response_parser.rs  # 响应解析、Token 统计
├── stream_parser.rs    # SSE 流解析
├── aimd_limiter.rs     # AIMD 限流器
├── circuit_breaker.rs  # 熔断器
├── balancer/           # 负载均衡策略
├── adaptor/            # 协议适配器 (OpenAI/Claude/Gemini)
└── scheduler/          # 请求调度
```

### Control Layer (crates/server)

**职责**：
- RESTful API
- 路由注册
- Handler
- 状态管理

**关键原则**：Handler 只做参数提取和响应构造，业务逻辑在 Service

**文件结构**：
```
crates/server/
├── routes/     # HTTP 路由注册
├── handlers/   # Axum Handler
├── state.rs    # AppState 管理
└── error.rs    # HTTP 错误响应
```

### Service Layer (crates/service)

**职责**：
- 计费
- 监控
- 用户管理
- Token 管理
- Channel 管理

**关键原则**：Service 不持有 Database，通过参数传入

**子 crate 结构**：
```
crates/service/
├── service-user      # 用户注册、登录、JWT
├── service-token     # Token 管理、验证
├── service-channel   # Channel CRUD、测速
├── service-billing   # 计费、价格缓存、成本计算
├── service-router-log # 请求日志
├── service-monitor   # 监控统计
├── service-group     # 用户组管理
├── service-models    # 模型管理
└── service-setting   # 系统设置
```

### Data Layer (crates/database)

**职责**：
- SQL 执行
- Schema 定义
- Placeholder 工具
- Repository trait

**关键原则**：使用 `ph()/phs()` 处理 SQLite/PostgreSQL 占位符差异

**子 crate 结构**：
```
crates/database/
├── database-user     # User 表操作
├── database-token    # Token 表操作
├── database-channel  # Channel 表操作
├── database-router   # Router 配置表
├── database-billing  # Billing 表操作
└── database-model    # Model 价格表
```

### Common Layer (crates/common)

**职责**：
- CrudRepository trait
- 跨 crate 纯工具

**关键原则**：不依赖任何其他 crate

---

## 五、跨层调用规则

### 允许的调用

| 调用方 | 被调用方 | 示例 |
|--------|----------|------|
| Server | Service | `UserService::get_all(&state.db)` |
| Service | Database | `UserRepository::find_by_id(db, id)` |
| Database | Common | `CrudRepository::save(db, entity)` |
| Router | service-billing | `PriceCache::get_price(model)` |
| Router | service-user | `UserService::resolve_traffic_class(user)` |

### 禁止的调用

| 调用方 | 被调用方 | 原因 |
|--------|----------|------|
| Service | Router | 反向依赖 |
| Database | Service | 反向依赖 |
| Common | 任何 crate | Common 是最底层 |
| Handler | Database | 跨层调用 |

---

## 六、架构检查

### 检查项

- [ ] 依赖方向正确（Server → Service → Database → Common）
- [ ] 未引入宪法例外外的反向依赖
- [ ] Handler 不直接调用 Database
- [ ] Service 不持有 Database 字段
- [ ] 文件落位正确（按层级归位）

### 检查命令

```bash
# 检查依赖方向
cargo tree -p burncloud-service-user | grep burncloud-router
# 应该无输出

# 检查跨层调用
grep -r "use burncloud_database" crates/server/src/handlers/
# 应该无输出
```
