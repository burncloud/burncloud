# burncloud-database-models

多实体模型数据库。管理 AI 模型、Channel、Ability、定价、视频任务和协议配置。

## 关键类型

| 类型 | 说明 |
|------|------|
| `ModelDatabase` | 聚合器,提供所有模型相关表的 `init(&db)` |
| `ChannelModel` | Channel CRUD |
| `AbilityModel` | 能力映射(Group → Model → Channel) |
| `PriceModel` | 标准定价 |
| `TieredPriceModel` | 分段定价(如 Qwen 按上下文长度) |
| `ProtocolConfigModel` | 动态协议适配器配置 |

## 目录结构

```
src/
├── lib.rs          — 聚合器,导出 9 个子模块
├── channel.rs      — ChannelModel
├── ability.rs      — AbilityModel
├── price.rs        — PriceModel
├── tiered_price.rs — TieredPriceModel
├── model.rs        — ModelInfo
├── token.rs        — Token 统计
├── video.rs        — VideoTask
└── protocol_config.rs — ProtocolConfigModel
```

## 依赖

- `burncloud-database`, `burncloud-common` — 核心抽象和共享类型
