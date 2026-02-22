# burncloud-router

数据平面 (Data Plane)，核心网关组件。

## 架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           burncloud-router                                   │
│                              (Data Plane)                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                        proxy_handler                                 │    │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │    │
│  │  │  Auth    │→│  Quota   │→│  Rate    │→│  Model   │→│  Proxy   │  │    │
│  │  │  Check   │ │  Check   │ │  Limit   │ │  Route   │ │  Logic   │  │    │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                      │                                       │
│            ┌─────────────────────────┼─────────────────────────┐            │
│            │                         │                         │            │
│            ▼                         ▼                         ▼            │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────────┐  │
│  │   balancer/      │    │    adaptor/      │    │  Core Modules        │  │
│  │  ┌────────────┐  │    │  ┌────────────┐  │    │  ┌────────────────┐  │  │
│  │  │RoundRobin  │  │    │  │ factory    │  │    │  │ billing        │  │  │
│  │  │Balancer    │  │    │  ├────────────┤  │    │  │ 计费模块       │  │  │
│  │  │负载均衡    │  │    │  │ dynamic    │  │    │  ├────────────────┤  │  │
│  │  └────────────┘  │    │  │ 动态适配   │  │    │  │ token_counter  │  │  │
│  └──────────────────┘    │  ├────────────┤  │    │  │ Token计数器    │  │  │
│                          │  │ detector   │  │    │  ├────────────────┤  │  │
│                          │  │ 版本检测   │  │    │  │ stream_parser  │  │  │
│                          │  ├────────────┤  │    │  │ 流式解析       │  │  │
│                          │  │ claude     │  │    │  ├────────────────┤  │  │
│                          │  │ gemini     │  │    │  │ channel_state  │  │  │
│                          │  │ vertex     │  │    │  │ 通道状态       │  │  │
│                          │  └────────────┘  │    │  ├────────────────┤  │  │
│                          └──────────────────┘    │  │ circuit_       │  │  │
│                                                  │  │ breaker        │  │  │
│                                                  │  │ 熔断器         │  │  │
│                                                  │  ├────────────────┤  │  │
│                                                  │  │ adaptive_limit │  │  │
│                                                  │  │ 自适应限流     │  │  │
│                                                  │  ├────────────────┤  │  │
│                                                  │  │ price_sync     │  │  │
│                                                  │  │ 价格同步       │  │  │
│                                                  │  └────────────────┘  │  │
│                                                  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 模块清单

| 模块 | 文件 | 职责 |
|------|------|------|
| **lib.rs** | `lib.rs` | 主入口，`create_router_app()`，`proxy_handler` |
| **billing** | `billing.rs` | 计费计算：分段、缓存、批处理、优先级 |
| **token_counter** | `token_counter.rs` | 线程安全的流式 Token 计数器 |
| **stream_parser** | `stream_parser.rs` | 多提供商流式响应解析 |
| **channel_state** | `channel_state.rs` | 通道/模型健康状态追踪 |
| **circuit_breaker** | `circuit_breaker.rs` | 熔断器实现 |
| **adaptive_limit** | `adaptive_limit.rs` | 自适应限流状态机 |
| **response_parser** | `response_parser.rs` | 多提供商响应解析 |
| **price_sync** | `price_sync.rs` | LiteLLM 价格同步 (每小时) |
| **notification** | `notification.rs` | Webhook 告警通知 |
| **model_router** | `model_router.rs` | 基于模型的路由 |
| **passthrough** | `passthrough.rs` | 流式透传逻辑 |
| **config** | `config.rs` | 路由配置类型 |
| **limiter** | `limiter.rs` | 基础限流器 |
| **pricing_loader** | `pricing_loader.rs` | 定价配置加载 |
| **exchange_rate** | `exchange_rate.rs` | 汇率管理 |

## 子模块

### adaptor/ - 协议适配器

| 文件 | 职责 |
|------|------|
| `factory.rs` | `DynamicAdaptorFactory`，DashMap 缓存 |
| `dynamic.rs` | 运行时协议适配配置 |
| `detector.rs` | API 版本废弃检测 |
| `mapping.rs` | JSON 字段映射 (JSONPath) |
| `claude.rs` | Anthropic 协议适配 |
| `gemini.rs` | Google Gemini 协议适配 |
| `vertex.rs` | Vertex AI 协议适配 |

### balancer/ - 负载均衡

| 文件 | 职责 |
|------|------|
| `mod.rs` | `RoundRobinBalancer` 轮询均衡 |

## 关键结构体

```
AppState
├── client: Client                      # HTTP 客户端
├── config: Arc<RwLock<RouterConfig>>   # 路由配置
├── db: Arc<Database>                   # 数据库
├── balancer: Arc<RoundRobinBalancer>   # 负载均衡器
├── limiter: Arc<RateLimiter>           # 限流器
├── circuit_breaker: Arc<CircuitBreaker> # 熔断器
├── model_router: Arc<ModelRouter>      # 模型路由器
├── channel_state_tracker               # 通道状态追踪
├── adaptor_factory                     # 动态适配器工厂
└── api_version_detector                # API 版本检测器
```

## 数据流

```
Request ──► Auth ──► Quota ──► RateLimit ──► ModelRouter
                                                    │
                      ┌─────────────────────────────┘
                      │
                      ▼
            ┌─────────────────┐
            │  CircuitBreaker │ ── Skip if open
            └────────┬────────┘
                     │
                     ▼
            ┌─────────────────┐
            │    Adaptor      │ ── Protocol conversion
            └────────┬────────┘
                     │
                     ▼
            ┌─────────────────┐
            │   Upstream      │ ── Forward request
            └────────┬────────┘
                     │
                     ▼
            ┌─────────────────┐
            │  Stream Parser  │ ── Parse tokens
            └────────┬────────┘
                     │
                     ▼
            ┌─────────────────┐
            │    Billing      │ ── Calculate cost
            └────────┬────────┘
                     │
                     ▼
              Response + Log
```

## 核心设计原则

**"Don't Touch the Body"** - 路由器是智能管道，不解析/修改请求体（认证除外）

## 依赖关系

```
burncloud-router
├── burncloud-common      # 共享类型
├── burncloud-database    # 数据库访问
│   ├── database-router   # 路由数据
│   └── database-models   # 价格模型
└── external: axum, reqwest, tokio
```
