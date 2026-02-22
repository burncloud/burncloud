# burncloud-database

持久化层，SQLx 数据库访问。

## 架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           burncloud-database                                 │
│                            (Persistence Layer)                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                         database.rs                                    │  │
│  │  ├── Database struct                                                   │  │
│  │  ├── create_default_database()                                        │  │
│  │  └── 连接池管理                                                        │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                      │                                       │
│            ┌─────────────────────────┼─────────────────────────┐            │
│            │                         │                         │            │
│            ▼                         ▼                         ▼            │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────────┐  │
│  │database-router   │    │ database-user    │    │ database-models      │  │
│  │                  │    │                  │    │                      │  │
│  │ RouterDatabase   │    │ UserDatabase     │    │ PriceModel           │  │
│  │ ├── validate_    │    │ ├── create_user  │    │ TieredPriceModel     │  │
│  │ │   token        │    │ ├── get_user     │    │                      │  │
│  │ ├── insert_log   │    │ ├── update_user  │    │ ├── get()            │  │
│  │ ├── get_         │    │ └── list_users   │    │ ├── set()            │  │
│  │ │   upstreams    │    │                  │    │ ├── delete()         │  │
│  │ └── ...          │    │                  │    │ └── has_tiered_      │  │
│  └──────────────────┘    └──────────────────┘    │     pricing()         │  │
│                                                  └──────────────────────┘  │
│                                                                              │
│  ┌──────────────────┐    ┌──────────────────┐                              │
│  │database-setting  │    │database-download │                              │
│  │                  │    │                  │                              │
│  │ 设置存储         │    │ 下载追踪         │                              │
│  └──────────────────┘    └──────────────────┘                              │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 子 Crate 清单

| Crate | 目录 | 职责 |
|-------|------|------|
| **database** | `crates/database/` | 核心数据库连接和 Schema |
| **database-router** | `crates/database-router/` | 路由数据操作 |
| **database-user** | `crates/database-user/` | 用户数据操作 |
| **database-models** | `crates/database-models/` | 模型/价格数据操作 |
| **database-setting** | `crates/database-setting/` | 设置存储 |
| **database-download** | `crates/database-download/` | 下载追踪 |

## 数据库表

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Database Schema                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  用户相关                                                                    │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                     │
│  │   users     │    │   tokens    │    │  recharges  │                     │
│  │─────────────│    │─────────────│    │─────────────│                     │
│  │ id          │    │ id          │    │ id          │                     │
│  │ username    │    │ user_id     │    │ user_id     │                     │
│  │ password    │    │ key         │    │ amount      │                     │
│  │ role        │    │ status      │    │ description │                     │
│  │ balance_usd │    │ name        │    │ created_at  │                     │
│  │ balance_cny │    │ expired_time│    └─────────────┘                     │
│  │ group       │    └─────────────┘                                        │
│  └─────────────┘                                                            │
│                                                                              │
│  路由相关                                                                    │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                     │
│  │  channels   │    │  abilities  │    │ router_logs │                     │
│  │─────────────│    │─────────────│    │─────────────│                     │
│  │ id          │    │ group       │    │ request_id  │                     │
│  │ type        │    │ model       │    │ user_id     │                     │
│  │ key         │    │ channel_id  │    │ path        │                     │
│  │ base_url    │    │ enabled     │    │ status_code │                     │
│  │ models      │    │ priority    │    │ latency_ms  │                     │
│  │ group       │    │ weight      │    │ prompt_tok  │                     │
│  │ priority    │    └─────────────┘    │ completion  │                     │
│  │ api_version │                       │ cost        │                     │
│  └─────────────┘                       └─────────────┘                     │
│                                                                              │
│  价格相关                                                                    │
│  ┌─────────────┐    ┌─────────────────┐    ┌─────────────────┐             │
│  │   prices    │    │ tiered_pricing  │    │ exchange_rates  │             │
│  │─────────────│    │─────────────────│    │─────────────────│             │
│  │ id          │    │ id              │    │ id              │             │
│  │ model       │    │ model           │    │ from_currency   │             │
│  │ currency    │    │ region          │    │ to_currency     │             │
│  │ input_price │    │ tier_start      │    │ rate (scaled)   │             │
│  │ output_price│    │ tier_end        │    │ updated_at      │             │
│  │ cache_*     │    │ input_price     │    └─────────────────┘             │
│  │ batch_*     │    │ output_price    │                                    │
│  │ priority_*  │    └─────────────────┘                                    │
│  │ region      │                                                           │
│  └─────────────┘                                                           │
│                                                                              │
│  配置相关                                                                    │
│  ┌─────────────────────┐                                                   │
│  │ protocol_configs    │                                                   │
│  │─────────────────────│                                                   │
│  │ id                  │                                                   │
│  │ channel_type        │                                                   │
│  │ api_version         │                                                   │
│  │ chat_endpoint       │                                                   │
│  │ request_mapping     │                                                   │
│  │ response_mapping    │                                                   │
│  └─────────────────────┘                                                   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 关键类型

### database-router

```rust
pub struct RouterDatabase;

impl RouterDatabase {
    pub async fn init(db: &Database) -> anyhow::Result<()>;
    pub async fn validate_token_detailed(...) -> Result<TokenValidationResult>;
    pub async fn insert_log(...) -> Result<()>;
    pub async fn get_all_upstreams(...) -> Result<Vec<DbUpstream>>;
    // ...
}

pub enum TokenValidationResult {
    Valid(TokenInfo),
    Expired,
    Invalid,
}
```

### database-models

```rust
pub struct PriceModel;

impl PriceModel {
    pub async fn get(db: &Database, model: &str, currency: &str, region: Option<&str>) -> Result<Option<Price>>;
    pub async fn set(db: &Database, input: &PriceInput) -> Result<()>;
    pub async fn delete(db: &Database, model: &str) -> Result<()>;
    pub fn calculate_cost(price: &Price, prompt: u64, completion: u64) -> i64;
}

pub struct TieredPriceModel;

impl TieredPriceModel {
    pub async fn get_tiers(...) -> Result<Vec<TieredPrice>>;
    pub async fn has_tiered_pricing(...) -> Result<bool>;
}
```

## 依赖关系

```
burncloud-database
├── database (核心)
│   └── schema.rs
├── database-router
├── database-user
├── database-models
├── database-setting
└── database-download

依赖: sqlx (SQLite/PostgreSQL)
```
