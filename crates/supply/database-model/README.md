# burncloud-database-model

`model_` 业务域数据库 crate，管理模型能力元数据（`model_capabilities` 表）。

## 关键类型

| 类型 | 说明 |
|------|------|
| `ModelDatabase` | Crate 控制器，占位实现（HuggingFace 风格元数据） |
| `ModelInfo`     | 模型元数据行类型 |

## 目录结构

```
src/
├── lib.rs               — 聚合器 + re-exports
├── common.rs            — current_timestamp() 工具
└── model_capability.rs  — ModelInfo / ModelDatabase
```

## 依赖

- `burncloud-database` — 核心抽象
