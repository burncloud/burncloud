# burncloud-service-redis

Redis 服务层。连接池管理和常用操作封装。

## 关键类型

| 类型 | 说明 |
|------|------|
| `RedisService` | Redis 连接池和操作 |
| `RedisError` | Redis 错误类型 |

## 依赖

- `redis`(tokio-comp) — Redis 客户端
- `burncloud-common` — 共享类型

## 注意

纯外部服务封装,无对应 database 子 crate。
