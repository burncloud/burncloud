# burncloud-commerce-billing

Commerce 域的 Billing Owner。把历史上的 `database-billing` 与
`service-billing` 收敛为一个纵向业务 crate。

## Domain Crate 10 问

1. **Owner 是谁？** Commerce / Billing。
2. **负责什么？** 价格持久化、价格缓存、usage 归一化、Token 计数和请求成本计算。
3. **Data Truth 是什么？** billing price / tiered price，以及由其派生的价格缓存和请求 cost。
4. **公开什么契约？** Pricing DTO 仍由 `burncloud-commerce-contracts` 定义；本 crate 公开 Billing 实现 API。
5. **可以依赖谁？** Commerce/Supply contracts、Platform Database 和通用运行时库。
6. **谁可以依赖它？** Interface/Traffic 的明确业务调用方；新增消费者需要架构审查。
7. **内部如何组织？** 业务模块在根层，SQL/rows 在私有 `repository/`。
8. **错误/日志/安全？** 延续现有 BillingError/DatabaseError 和 tracing 行为；不改变金额单位。
9. **如何证明正确？** 原 database/service 两组测试全部迁移，SQLite/Postgres、cache、multimodal、batch/priority 行为保持。
10. **正确边界是什么？** Commerce 计算请求成本；Identity 管 credential quota state；Traffic 管 raw router logs 和 routing projection。

## 不拥有

- Traffic `router_logs` 原始日志 Data Truth
- Identity `quota_limit / used_quota` credential state
- Traffic `order_type / price_cap_nanodollars`
- Platform 数据库连接/迁移基础设施

## 目录

```text
src/
├── cache.rs
├── calculator.rs
├── counter.rs
├── error.rs
├── types.rs
├── usage/
└── repository/
    ├── billing_price.rs
    ├── billing_tiered_price.rs
    ├── common.rs
    └── rows.rs
```

迁移不变量：

```text
Before behavior == After behavior
Before data == After data
Before public business contract == After public business contract
```
