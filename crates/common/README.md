# burncloud-common

全项目共享的类型定义、常量和工具函数。无外部 crate 依赖,是依赖树的最底层。

## 为什么存在

所有层级都需要共享的基础类型(如 ChannelType、Currency、纳美元转换函数),避免在多个 crate 中重复定义。

## 关键类型

| 类型 | 说明 |
|------|------|
| `ChannelType` | Channel 类型枚举(OpenAI=1, Azure=3, Anthropic=14, Gemini=24...) |
| `Currency` | 币种枚举(USD, CNY, EUR) |
| `dollars_to_nano()` | 美元 → 纳美元 |
| `nano_to_dollars()` | 纳美元 → 美元 |
| `calculate_cost_safe()` | 安全费用计算 |
| `NANO_PER_DOLLAR` | 纳美元常量(1_000_000_000) |

## 依赖

无 burncloud crate 依赖。仅依赖 `serde`, `strum` 等基础库。

## 目录结构

```
src/
├── lib.rs              — 导出
├── types.rs            — ChannelType, Currency 等类型
├── constants.rs        — 全局常量
├── error.rs            — 共享错误类型
├── config.rs           — 配置工具
├── price_u64.rs        — 价格计算(u64 版)
├── pricing_config.rs   — 定价配置(内置模型价格)
└── utils.rs            — 通用工具函数
```
