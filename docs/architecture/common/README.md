# burncloud-common

共享类型和工具，被所有 crate 依赖。

## 架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           burncloud-common                                   │
│                            (Shared Types)                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                            types.rs                                     │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │ │
│  │  │   Currency   │  │MultiCurrency │  │  ExchangeRate │                  │ │
│  │  │──────────────│  │    Price     │  │──────────────│                  │ │
│  │  │ USD          │  │──────────────│  │ from_currency│                  │ │
│  │  │ CNY          │  │ currency     │  │ to_currency  │                  │ │
│  │  │ EUR          │  │ input_price  │  │ rate (scaled)│                  │ │
│  │  └──────────────┘  │ output_price │  └──────────────┘                  │ │
│  │                    └──────────────┘                                     │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │ │
│  │  │    User      │  │    Token     │  │   Channel    │                  │ │
│  │  │──────────────│  │──────────────│  │──────────────│                  │ │
│  │  │ id           │  │ id           │  │ id           │                  │ │
│  │  │ username     │  │ user_id      │  │ type         │                  │ │
│  │  │ balance_usd  │  │ key          │  │ key          │                  │ │
│  │  │ balance_cny  │  │ status       │  │ base_url     │                  │ │
│  │  │ group        │  │ expired_time │  │ models       │                  │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘                  │ │
│  │                                                                         │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │ │
│  │  │   Ability    │  │    Price     │  │ TieredPrice  │                  │ │
│  │  │──────────────│  │──────────────│  │──────────────│                  │ │
│  │  │ group        │  │ model        │  │ model        │                  │ │
│  │  │ model        │  │ currency     │  │ region       │                  │ │
│  │  │ channel_id   │  │ input_price  │  │ tier_start   │                  │ │
│  │  │ enabled      │  │ output_price │  │ tier_end     │                  │ │
│  │  │ priority     │  │ cache_*      │  │ input_price  │                  │ │
│  │  └──────────────┘  │ batch_*      │  │ output_price │                  │ │
│  │                    │ priority_*   │  └──────────────┘                  │ │
│  │                    │ region       │                                    │ │
│  │                    └──────────────┘                                    │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐                 │
│  │ pricing_config │  │   price_u64    │  │     error      │                 │
│  │                │  │                │  │                │                 │
│  │ PricingConfig  │  │dollars_to_nano │  │ AppError       │                 │
│  │ ModelPricing   │  │nano_to_dollars │  │                │                 │
│  │ TieredPrice    │  │NANO_PER_DOLLAR │  │                │                 │
│  │ CachePricing   │  │RATE_SCALE      │  │                │                 │
│  └────────────────┘  └────────────────┘  └────────────────┘                 │
│                                                                              │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐                 │
│  │     config     │  │   constants    │  │     utils      │                 │
│  │                │  │                │  │                │                 │
│  │ 配置常量       │  │ 应用常量       │  │ 工具函数       │                 │
│  └────────────────┘  └────────────────┘  └────────────────┘                 │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 模块清单

| 模块 | 文件 | 职责 |
|------|------|------|
| **types** | `types.rs` | 核心数据类型定义 |
| **pricing_config** | `pricing_config.rs` | 定价配置 Schema |
| **price_u64** | `price_u64.rs` | 纳美元价格处理 |
| **config** | `config.rs` | 配置常量 |
| **constants** | `constants.rs` | 应用常量 |
| **error** | `error.rs` | 错误类型 |
| **utils** | `utils.rs` | 工具函数 |

## 核心类型

### Currency

```rust
pub enum Currency {
    USD,  // $
    CNY,  // ¥
    EUR,  // €
}
```

### Price (纳美元精度)

```rust
pub struct Price {
    pub model: String,
    pub currency: String,
    pub input_price: i64,           // 纳美元 (9位小数)
    pub output_price: i64,
    pub cache_read_input_price: Option<i64>,
    pub cache_creation_input_price: Option<i64>,
    pub batch_input_price: Option<i64>,
    pub batch_output_price: Option<i64>,
    pub priority_input_price: Option<i64>,
    pub priority_output_price: Option<i64>,
    pub audio_input_price: Option<i64>,
    pub region: Option<String>,
    // ...
}
```

### User (双币种钱包)

```rust
pub struct User {
    pub id: String,
    pub username: String,
    pub role: i32,           // 1: Common, 10: Admin, 100: Root
    pub balance_usd: i64,    // USD 余额 (纳美元)
    pub balance_cny: i64,    // CNY 余额 (纳美元)
    pub group: String,
    // ...
}
```

## 价格计算工具

```rust
// 纳美元常量
pub const NANO_PER_DOLLAR: i64 = 1_000_000_000;
pub const RATE_SCALE: i64 = 1_000_000_000;

// 转换函数
pub fn dollars_to_nano(dollars: f64) -> i64;
pub fn nano_to_dollars(nano: i64) -> f64;
pub fn rate_to_scaled(rate: f64) -> i64;
pub fn scaled_to_rate(scaled: i64) -> f64;
pub fn calculate_cost_safe(price_per_million: i64, tokens: u64) -> f64;
```

## ChannelType 枚举

```rust
pub enum ChannelType {
    OpenAI = 1,
    Azure = 3,
    Anthropic = 14,
    Gemini = 24,
    VertexAi = 41,
    // ... 50+ 渠道类型
}
```

## 使用示例

```rust
use burncloud_common::{
    Currency,
    types::{User, Token, Channel, Price},
    price_u64::{dollars_to_nano, nano_to_dollars},
};

// 价格转换
let price = dollars_to_nano(0.002);  // $0.002 → 2_000_000 纳美元
let display = nano_to_dollars(price); // → 0.002
```

## 依赖关系

```
burncloud-common (基础层，无内部依赖)

被以下 crate 依赖:
├── burncloud-router
├── burncloud-server
├── burncloud-database
├── burncloud-client
├── burncloud-cli
└── ...
```
