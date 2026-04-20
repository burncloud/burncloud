# burncloud-router

LLM 请求代理引擎。接收客户端请求,路由到上游 Provider,处理协议转换、负载均衡、限流、熔断和计费。

## 为什么存在

BurnCloud 需要一个高性能的数据平面,在多个 LLM Provider 之间智能路由请求。这个 crate 封装了所有代理逻辑,让 Server 层只需关注管理 API。

## 关键类型

| 类型 | 说明 |
|------|------|
| `AppState` | Router 运行时状态(client, config, balancer, limiter, circuit_breaker, price_cache...) |
| `RouterConfig` | 动态路由配置(Upstream + Group 列表) |
| `Upstream` / `Group` | 上游 Provider 和分组定义 |
| `ChannelAdaptor` | 协议适配器 trait(OpenAI, Claude, Gemini, Vertex...) |
| `CircuitBreaker` | 熔断器(5 次失败触发,30 秒冷却) |
| `RateLimiter` | 令牌桶限流 |
| `ModelRouter` | 模型 → Channel 路由逻辑 |
| `AdvancedPricing` | 多模态定价(标准/缓存/批量/优先级/音频) |
| `StreamingTokenCounter` | 线程安全流式 Token 计数 |

## 依赖

- `burncloud-database`, `burncloud-database-router`, `burncloud-database-model` — 数据持久化
- `burncloud-common` — 共享类型
- `burncloud-service-billing` — 计费计算
- `burncloud-router-aws` — AWS SigV4 签名

## 目录结构

```
src/
├── lib.rs              — 核心入口, Handler, 启动逻辑
├── state.rs            — AppState 定义
├── config.rs           — RouterConfig, Upstream, Group, AuthType
├── model_router.rs     — 模型路由
├── billing.rs          — 高级定价计算
├── token_counter.rs    — 流式 Token 计数
├── price_sync.rs       — 多源价格同步
├── circuit_breaker.rs  — 熔断器
├── limiter.rs          — 令牌桶限流
├── channel_state.rs    — Channel 健康追踪
├── adaptive_limit.rs   — 自适应限流
├── exchange_rate.rs    — 汇率处理
├── adaptor/            — 协议适配器(OpenAI, Claude, Gemini, Vertex, ZAI, Dynamic)
├── balancer/           — Round-Robin 负载均衡
└── proxy*.rs           — 代理请求处理
```

## 核心原则

**"Don't Touch the Body"** — Router 不修改请求/响应体。只做路由、协议转换、计费、限流。

## Sources

- `crates/router/src/lib.rs` — 完整请求处理流水线
- `docs/backend/router-guide.md` — 开发指南
