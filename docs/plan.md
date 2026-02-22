十五、自定义价格数据源（方案 B：多货币独立定价）

### 问题背景

LiteLLM 价格数据库存在以下限制：

| 限制 | 影响 |
|------|------|
| 仅支持 USD 货币 | 无法表达 Qwen 国内版的人民币价格 |
| 缺少阶梯定价数据 | Qwen/DeepSeek 阶梯定价无法自动同步 |
| 缺少区域定价信息 | 国内版/海外版价格差异无法区分 |
| 更新滞后 | 新模型发布后价格更新不及时 |

**解决方案**: 建立独立的多货币价格数据源，支持：
1. 多货币定价（USD/CNY/EUR）
2. 阶梯定价配置
3. 区域定价差异
4. 手动维护 + 社区贡献

### 价格数据源架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                    BurnCloud 价格数据源架构                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  数据源优先级:                                                       │
│  1. 本地配置文件 (pricing.json) - 最高优先级，手动维护               │
│  2. BurnCloud 社区价格库 - 次优先级，社区维护                         │
│  3. LiteLLM 价格库 - 最低优先级，仅作为 USD 参考                     │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     pricing.json 结构                        │   │
│  │                                                             │   │
│  │  {                                                          │   │
│  │    "version": "1.0",                                        │   │
│  │    "updated_at": "2024-01-15T10:00:00Z",                   │   │
│  │    "models": {                                              │   │
│  │      "qwen-max": {                                          │   │
│  │        "pricing": {                                         │   │
│  │          "USD": {                                           │   │
│  │            "input_price": 0.0012,                          │   │
│  │            "output_price": 0.006,                          │   │
│  │            "source": "international"                        │   │
│  │          },                                                 │   │
│  │          "CNY": {                                           │   │
│  │            "input_price": 0.002,                           │   │
│  │            "output_price": 0.006,                          │   │
│  │            "source": "cn"                                   │   │
│  │          }                                                  │   │
│  │        },                                                   │   │
│  │        "tiered_pricing": {                                  │   │
│  │          "USD": [                                           │   │
│  │            {"tier_start": 0, "tier_end": 32000,             │   │
│  │             "input_price": 1.2, "output_price": 6.0},       │   │
│  │            {"tier_start": 32000, "tier_end": 128000,        │   │
│  │             "input_price": 2.4, "output_price": 12.0}       │   │
│  │          ],                                                 │   │
│  │          "CNY": [                                           │   │
│  │            {"tier_start": 0, "tier_end": 32000,             │   │
│  │             "input_price": 0.359, "output_price": 1.434}    │   │
│  │          ]                                                  │   │
│  │        },                                                   │   │
│  │        "cache_pricing": {                                   │   │
│  │          "USD": {                                           │   │
│  │            "cache_read_price": 0.00012,                    │   │
│  │            "cache_creation_price": 0.0015                  │   │
│  │          }                                                  │   │
│  │        },                                                   │   │
│  │        "metadata": {                                        │   │
│  │          "context_window": 252000,                         │   │
│  │          "max_output_tokens": 8192,                        │   │
│  │          "supports_vision": false,                         │   │
│  │          "supports_function_calling": true                 │   │
│  │        }                                                    │   │
│  │      }                                                      │   │
│  │    }                                                        │   │
│  │  }                                                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### pricing.json 完整 Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "BurnCloud Pricing Configuration",
  "type": "object",
  "required": ["version", "updated_at", "models"],
  "properties": {
    "version": {
      "type": "string",
      "pattern": "^\\d+\\.\\d+$",
      "description": "Schema version, e.g. '1.0'"
    },
    "updated_at": {
      "type": "string",
      "format": "date-time",
      "description": "ISO 8601 timestamp of last update"
    },
    "source": {
      "type": "string",
      "enum": ["local", "community", "litellm"],
      "default": "local",
      "description": "Data source identifier"
    },
    "models": {
      "type": "object",
      "additionalProperties": {
        "$ref": "#/definitions/ModelPricing"
      }
    }
  },
  "definitions": {
    "ModelPricing": {
      "type": "object",
      "required": ["pricing"],
      "properties": {
        "pricing": {
          "type": "object",
          "description": "Standard pricing by currency",
          "additionalProperties": {
            "$ref": "#/definitions/CurrencyPricing"
          }
        },
        "tiered_pricing": {
          "type": "object",
          "description": "Tiered pricing by currency",
          "additionalProperties": {
            "type": "array",
            "items": {
              "$ref": "#/definitions/TieredPrice"
            }
          }
        },
        "cache_pricing": {
          "type": "object",
          "description": "Cache pricing by currency",
          "additionalProperties": {
            "$ref": "#/definitions/CachePricing"
          }
        },
        "batch_pricing": {
          "type": "object",
          "description": "Batch API pricing by currency",
          "additionalProperties": {
            "$ref": "#/definitions/BatchPricing"
          }
        },
        "metadata": {
          "$ref": "#/definitions/ModelMetadata"
        }
      }
    },
    "CurrencyPricing": {
      "type": "object",
      "required": ["input_price", "output_price"],
      "properties": {
        "input_price": {
          "type": "number",
          "minimum": 0,
          "description": "Price per 1M input tokens"
        },
        "output_price": {
          "type": "number",
          "minimum": 0,
          "description": "Price per 1M output tokens"
        },
        "source": {
          "type": "string",
          "description": "Pricing source region, e.g. 'cn', 'international'"
        }
      }
    },
    "TieredPrice": {
      "type": "object",
      "required": ["tier_start", "input_price", "output_price"],
      "properties": {
        "tier_start": {
          "type": "integer",
          "minimum": 0,
          "description": "Start of tier in tokens"
        },
        "tier_end": {
          "type": "integer",
          "minimum": 0,
          "description": "End of tier in tokens, null means unlimited"
        },
        "input_price": {
          "type": "number",
          "minimum": 0,
          "description": "Price per 1M input tokens in this tier"
        },
        "output_price": {
          "type": "number",
          "minimum": 0,
          "description": "Price per 1M output tokens in this tier"
        }
      }
    },
    "CachePricing": {
      "type": "object",
      "properties": {
        "cache_read_price": {
          "type": "number",
          "minimum": 0,
          "description": "Price per 1M cache-read tokens"
        },
        "cache_creation_price": {
          "type": "number",
          "minimum": 0,
          "description": "Price per 1M cache-creation tokens"
        }
      }
    },
    "BatchPricing": {
      "type": "object",
      "properties": {
        "input_price": {
          "type": "number",
          "minimum": 0,
          "description": "Batch API input price per 1M tokens"
        },
        "output_price": {
          "type": "number",
          "minimum": 0,
          "description": "Batch API output price per 1M tokens"
        }
      }
    },
    "ModelMetadata": {
      "type": "object",
      "properties": {
        "context_window": {
          "type": "integer",
          "description": "Maximum context window in tokens"
        },
        "max_output_tokens": {
          "type": "integer",
          "description": "Maximum output tokens"
        },
        "supports_vision": {
          "type": "boolean",
          "default": false
        },
        "supports_function_calling": {
          "type": "boolean",
          "default": false
        },
        "supports_streaming": {
          "type": "boolean",
          "default": true
        },
        "provider": {
          "type": "string",
          "description": "Model provider, e.g. 'openai', 'anthropic', 'alibaba'"
        }
      }
    }
  }
}
```

### 价格数据源管理

#### 1. 本地配置文件

```bash
# 配置文件位置
config/pricing.json          # 主配置文件
config/pricing.override.json # 覆盖配置（可选，优先级更高）

# CLI 管理
burncloud pricing import config/pricing.json
burncloud pricing export > config/pricing.json
burncloud pricing validate config/pricing.json
```

#### 2. 社区价格库

```
GitHub: burncloud/pricing-data

结构:
├── pricing/
│   ├── v1/
│   │   ├── openai.json       # OpenAI 系列模型
│   │   ├── anthropic.json    # Claude 系列模型
│   │   ├── alibaba.json      # 通义千问系列
│   │   ├── baidu.json        # 文心一言系列
│   │   ├── deepseek.json     # DeepSeek 系列
│   │   └── index.json        # 索引文件
│   └── latest.json           # 最新聚合文件
└── schemas/
    └── pricing.schema.json   # JSON Schema
```

#### 3. 同步策略

```
┌─────────────────────────────────────────────────────────────────────┐
│                      价格同步优先级策略                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  优先级 1: 本地覆盖配置 (pricing.override.json)                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 用途: 紧急价格调整、测试价格、客户定制价格                     │   │
│  │ 更新: 手动编辑                                               │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ↓                                      │
│  优先级 2: 本地主配置 (pricing.json)                                │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 用途: 生产环境标准价格                                        │   │
│  │ 更新: 手动编辑 + CLI 导入                                     │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ↓                                      │
│  优先级 3: 社区价格库 (GitHub)                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 用途: 社区维护的通用价格                                       │   │
│  │ 更新: 每日自动同步 + 手动触发                                 │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ↓                                      │
│  优先级 4: LiteLLM (仅 USD 参考)                                    │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 用途: 新模型 USD 价格参考                                      │   │
│  │ 更新: 每小时同步                                              │   │
│  │ 注意: 仅用于缺失价格时的回退                                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 数据库表设计（方案 B）

```sql
-- 多货币价格表（替代原有 prices 表）
CREATE TABLE prices_v2 (
    id INTEGER PRIMARY KEY,
    model TEXT NOT NULL,
    currency TEXT NOT NULL,           -- 'USD', 'CNY', 'EUR'

    -- 标准定价
    input_price REAL NOT NULL,
    output_price REAL NOT NULL,

    -- 缓存定价
    cache_read_input_price REAL,
    cache_creation_input_price REAL,

    -- 批量定价
    batch_input_price REAL,
    batch_output_price REAL,

    -- 优先级定价
    priority_input_price REAL,
    priority_output_price REAL,

    -- 音频定价
    audio_input_price REAL,

    -- 元数据
    source TEXT,                      -- 'local', 'community', 'litellm'
    region TEXT,                      -- 'cn', 'international', 'us', 'eu'
    context_window INTEGER,
    max_output_tokens INTEGER,
    supports_vision BOOLEAN DEFAULT FALSE,
    supports_function_calling BOOLEAN DEFAULT FALSE,

    -- 审计
    synced_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(model, currency, region)
);

-- 创建索引
CREATE INDEX idx_prices_v2_model ON prices_v2(model);
CREATE INDEX idx_prices_v2_currency ON prices_v2(currency);
CREATE INDEX idx_prices_v2_model_currency ON prices_v2(model, currency);

-- 阶梯定价表（保持不变，增加货币支持）
ALTER TABLE tiered_pricing ADD COLUMN currency TEXT DEFAULT 'USD';

-- 迁移数据
INSERT INTO prices_v2 (model, currency, input_price, output_price, source)
SELECT model, 'USD', input_price, output_price, 'litellm'
FROM prices;
```

### 同步服务实现

```rust
/// 价格同步服务
pub struct PriceSyncServiceV2 {
    db: Database,
    http_client: reqwest::Client,

    // 数据源配置
    local_config_path: PathBuf,
    override_config_path: PathBuf,
    community_repo_url: String,
    litellm_url: String,

    // 同步状态
    last_community_sync: DateTime<Utc>,
    last_litellm_sync: DateTime<Utc>,
}

impl PriceSyncServiceV2 {
    /// 同步所有价格源（按优先级）
    pub async fn sync_all(&mut self) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        // 1. 本地覆盖配置（最高优先级）
        if let Some(override_prices) = self.load_local_override().await? {
            result.merge(self.apply_prices(&override_prices, "local_override").await?);
        }

        // 2. 本地主配置
        if let Some(local_prices) = self.load_local_config().await? {
            result.merge(self.apply_prices(&local_prices, "local").await?);
        }

        // 3. 社区价格库（每日更新）
        if self.should_sync_community() {
            if let Some(community_prices) = self.fetch_community_prices().await? {
                result.merge(self.apply_prices(&community_prices, "community").await?);
                self.last_community_sync = Utc::now();
            }
        }

        // 4. LiteLLM（仅 USD，用于回退）
        if self.should_sync_litellm() {
            if let Some(litellm_prices) = self.fetch_litellm_prices().await? {
                result.merge(self.apply_litellm_prices(&litellm_prices).await?);
                self.last_litellm_sync = Utc::now();
            }
        }

        Ok(result)
    }

    /// 从本地配置文件加载价格
    async fn load_local_config(&self) -> Result<Option<PricingConfig>> {
        let path = &self.local_config_path;
        if !path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(path).await?;
        let config: PricingConfig = serde_json::from_str(&content)?;

        // 验证 schema
        self.validate_config(&config)?;

        Ok(Some(config))
    }

    /// 从社区仓库获取价格
    async fn fetch_community_prices(&self) -> Result<Option<PricingConfig>> {
        let url = format!("{}/pricing/latest.json", self.community_repo_url);

        let response = self.http_client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            warn!("Failed to fetch community prices: {}", response.status());
            return Ok(None);
        }

        let config: PricingConfig = response.json().await?;
        Ok(Some(config))
    }

    /// 应用价格到数据库（使用 UPSERT）
    async fn apply_prices(&self, config: &PricingConfig, source: &str) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        for (model, pricing) in &config.models {
            for (currency, price) in &pricing.pricing {
                self.upsert_price(model, currency, price, source).await?;
                result.models_updated += 1;
            }

            // 同步阶梯定价
            if let Some(tiered) = &pricing.tiered_pricing {
                for (currency, tiers) in tiered {
                    self.upsert_tiered_pricing(model, currency, tiers).await?;
                    result.tiers_updated += tiers.len();
                }
            }
        }

        result.source = source.to_string();
        Ok(result)
    }
}

/// 同步结果
#[derive(Debug, Default)]
pub struct SyncResult {
    pub source: String,
    pub models_updated: usize,
    pub tiers_updated: usize,
    pub errors: Vec<String>,
}
```

### CLI 命令

```bash
# 价格管理
burncloud pricing sync                    # 同步所有价格源
burncloud pricing sync --source community # 仅同步社区价格
burncloud pricing sync --source litellm   # 仅同步 LiteLLM

burncloud pricing import <file.json>      # 导入价格文件
burncloud pricing export [--format json]  # 导出当前价格
burncloud pricing validate <file.json>    # 验证价格文件格式

burncloud pricing show <model>            # 显示模型价格（所有货币）
burncloud pricing show <model> --currency CNY  # 显示指定货币价格

burncloud pricing set <model> \
    --currency CNY \
    --input-price 0.002 \
    --output-price 0.006 \
    --region cn

# 阶梯定价管理
burncloud pricing set-tier <model> \
    --currency CNY \
    --tier-start 0 \
    --tier-end 32000 \
    --input-price 0.359 \
    --output-price 1.434

burncloud pricing list-tiers <model> [--currency CNY]

# 同步状态
burncloud pricing status                  # 显示同步状态
burncloud pricing status --verbose        # 显示详细信息
```

### 示例价格文件

```json
{
  "version": "1.0",
  "updated_at": "2024-01-15T10:00:00Z",
  "source": "local",
  "models": {
    "gpt-4-turbo": {
      "pricing": {
        "USD": {
          "input_price": 10.0,
          "output_price": 30.0,
          "source": "openai"
        },
        "CNY": {
          "input_price": 72.0,
          "output_price": 216.0,
          "source": "converted"
        }
      },
      "cache_pricing": {
        "USD": {
          "cache_read_input_price": 1.0,
          "cache_creation_input_price": 1.25
        }
      },
      "metadata": {
        "context_window": 128000,
        "max_output_tokens": 4096,
        "supports_vision": true,
        "supports_function_calling": true,
        "provider": "openai"
      }
    },
    "qwen-max": {
      "pricing": {
        "USD": {
          "input_price": 1.2,
          "output_price": 6.0,
          "source": "international"
        },
        "CNY": {
          "input_price": 0.359,
          "output_price": 1.434,
          "source": "cn"
        }
      },
      "tiered_pricing": {
        "USD": [
          {"tier_start": 0, "tier_end": 32000, "input_price": 1.2, "output_price": 6.0},
          {"tier_start": 32000, "tier_end": 128000, "input_price": 2.4, "output_price": 12.0},
          {"tier_start": 128000, "tier_end": 252000, "input_price": 3.0, "output_price": 15.0}
        ],
        "CNY": [
          {"tier_start": 0, "tier_end": 32000, "input_price": 0.359, "output_price": 1.434},
          {"tier_start": 32000, "tier_end": 128000, "input_price": 0.574, "output_price": 2.294},
          {"tier_start": 128000, "tier_end": 252000, "input_price": 1.004, "output_price": 4.014}
        ]
      },
      "metadata": {
        "context_window": 252000,
        "max_output_tokens": 8192,
        "supports_vision": false,
        "supports_function_calling": true,
        "provider": "alibaba"
      }
    },
    "claude-3-5-sonnet-20241022": {
      "pricing": {
        "USD": {
          "input_price": 3.0,
          "output_price": 15.0
        }
      },
      "cache_pricing": {
        "USD": {
          "cache_read_input_price": 0.30,
          "cache_creation_input_price": 3.75
        }
      },
      "metadata": {
        "context_window": 200000,
        "max_output_tokens": 8192,
        "supports_vision": true,
        "supports_function_calling": true,
        "provider": "anthropic"
      }
    }
  }
}
```

### 实现优先级

| 优先级 | 任务 | 说明 |
|--------|------|------|
| P1 | 定义 pricing.json Schema | 建立标准格式 |
| P1 | 实现 PricingConfig 数据结构 | Rust 结构体映射 |
| P1 | 创建 prices_v2 数据库表 | 多货币支持 |
| P2 | 实现本地配置文件加载 | 最高优先级数据源 |
| P2 | 实现价格同步服务 V2 | 多数据源同步 |
| P2 | CLI 价格管理命令 | 导入/导出/设置 |
| P3 | 社区价格库同步 | GitHub 数据源 |
| P3 | 价格变更审计日志 | 记录所有变更 |

### 相关代码位置

- 价格同步: `crates/router/src/price_sync.rs` (升级为 V2)
- 价格模型: `crates/database-models/src/lib.rs`
- 数据库结构: `crates/database/src/schema.rs`
- CLI 命令: `crates/cli/src/pricing.rs`
- 配置文件: `config/pricing.json` (新建)

---
十六、双币钱包多货币充值与计费方案

### 问题背景

客户充值可能使用人民币或美元：
- 美国模型（OpenAI, Claude）原生 USD 定价
- 中国模型（Qwen, DeepSeek）原生 CNY 定价
- 汇率波动导致无论如何统一转换，都会产生价格对不齐的问题

**选定方案：双币钱包 + 智能扣费**

核心优势：
1. 用户充值多少就是多少，不受汇率波动影响
2. 资金池分离，平台无汇率风险
3. 跨币种消费时才发生换汇，最小化汇率影响

### 核心设计

#### 用户余额结构

```rust
pub struct UserBalance {
    pub balance_usd: f64,           // 美元余额
    pub balance_cny: f64,           // 人民币余额
    pub preferred_currency: Currency,  // 显示偏好 (USD/CNY)
}
```

#### 扣费优先级逻辑

```
美国模型 (pricing_region = "international"):
  → 优先扣 USD 余额
  → USD 不足时，CNY 按实时汇率转换补足

中国模型 (pricing_region = "cn"):
  → 优先扣 CNY 余额
  → CNY 不足时，USD 按实时汇率转换补足

通用模型 (pricing_region = NULL):
  → 按用户 preferred_currency 决定
```

### 数据库修改

#### users 表

```sql
-- 添加双币余额字段
ALTER TABLE users ADD COLUMN balance_usd REAL DEFAULT 0.0;
ALTER TABLE users ADD COLUMN balance_cny REAL DEFAULT 0.0;

-- 数据迁移：将现有 quota 转换为 USD 余额
-- 假设 500000 quota = $1
UPDATE users SET balance_usd = quota / 500000.0 WHERE quota > 0;
```

#### recharges 表

```sql
-- 添加货币类型
ALTER TABLE recharges ADD COLUMN currency VARCHAR(10) DEFAULT 'USD';
ALTER TABLE recharges ADD COLUMN original_amount REAL;  -- 充值原始金额
```

#### 新增 balance_logs 表

```sql
CREATE TABLE IF NOT EXISTS balance_logs (
    id INTEGER PRIMARY KEY,
    user_id TEXT NOT NULL,
    currency TEXT NOT NULL,           -- 'USD' | 'CNY'
    amount REAL NOT NULL,             -- 变动金额（正负）
    balance_after REAL NOT NULL,      -- 变动后余额
    reason TEXT,                      -- 原因：充值/消费/退款/换汇
    model TEXT,                       -- 消费的模型
    request_id TEXT,                  -- 关联请求
    exchange_rate REAL,               -- 换汇时的汇率
    created_at INTEGER
);

CREATE INDEX idx_balance_logs_user ON balance_logs(user_id);
CREATE INDEX idx_balance_logs_created ON balance_logs(created_at);
```

### 代码修改清单

#### P0 - 核心修改

| 文件 | 修改内容 |
|------|----------|
| `crates/common/src/types.rs` (行 321-347) | User 结构体添加 balance_usd, balance_cny 字段 |
| `crates/database/crates/database-user/src/lib.rs` | DbUser 添加双币字段，修改 CRUD 函数 |
| `crates/database/crates/database-router/src/lib.rs` (行 642-708) | 重写 deduct_quota 为 deduct_dual_currency |
| `crates/router/src/lib.rs` (行 577-590) | 扣费时根据 pricing_region 选择余额 |

#### P1 - API 修改

| 文件 | 修改内容 |
|------|----------|
| `crates/server/src/api/user.rs` (行 43-58) | TopupDto 添加 currency 字段，充值逻辑支持双币 |

#### P2 - 模型区域判断

| 文件 | 修改内容 |
|------|----------|
| `crates/router/src/lib.rs` (行 632-661) | 路由选择时传递 pricing_region |

### 核心扣费逻辑

```rust
// 文件: crates/database/crates/database-router/src/lib.rs

pub async fn deduct_dual_currency(
    db: &Database,
    user_id: &str,
    token: &str,
    usd_cost: f64,
    cny_cost: f64,
    pricing_region: &str,
) -> Result<DeductResult> {
    // 1. 开始事务
    // 2. 根据 pricing_region 决定优先扣减的币种
    // 3. 检查余额充足性
    // 4. 扣减余额（必要时换汇）
    // 5. 记录 balance_logs
    // 6. 同时扣减 token 配额（兼容现有逻辑）
}
```

### 余额不足处理（请求前同步检查）

```rust
// 文件: crates/router/src/lib.rs (行 381-387)

async fn check_balance_sufficient(
    db: &Database,
    user_id: &str,
    estimated_cost_usd: f64,
    estimated_cost_cny: f64,
    pricing_region: &str,
) -> Result<bool> {
    let user = UserModel::get(db, user_id).await?;

    match pricing_region {
        "cn" => {
            // 中国模型：检查 CNY 余额，或 USD 等值
            if user.balance_cny >= estimated_cost_cny {
                return Ok(true);
            }
            let usd_equiv = estimated_cost_cny / get_rate("CNY", "USD");
            Ok(user.balance_usd >= usd_equiv)
        }
        _ => {
            // 美国模型：检查 USD 余额，或 CNY 等值
            if user.balance_usd >= estimated_cost_usd {
                return Ok(true);
            }
            let cny_equiv = estimated_cost_usd * get_rate("USD", "CNY");
            Ok(user.balance_cny >= cny_equiv)
        }
    }
}
```

### 价格选择策略

```rust
// 文件: crates/router/src/billing.rs 或新建 pricing_selector.rs

pub fn select_price_for_region(
    model: &str,
    region: &str,
    db: &Database,
) -> PriceResult {
    let (preferred_currency, fallback_currency) = match region {
        "cn" => (Currency::CNY, Currency::USD),
        _ => (Currency::USD, Currency::CNY),
    };

    // 优先获取偏好货币的价格
    if let Some(price) = PriceV2Model::get(db, model, preferred_currency, region).await {
        return price;
    }

    // 回退到另一种货币
    if let Some(price) = PriceV2Model::get(db, model, fallback_currency, region).await {
        return price.with_exchange_rate(get_current_rate());
    }

    // 最终回退到 prices 表
    PriceModel::get(db, model).await
}
```

### 实现步骤

#### Phase 1: 数据库迁移
- [ ] 添加 `balance_usd`, `balance_cny` 字段到 users 表
- [ ] 添加 `currency` 字段到 recharges 表
- [ ] 创建 `balance_logs` 表
- [ ] 数据迁移：现有 quota 转换为 USD 余额

#### Phase 2: 类型定义更新
- [ ] 修改 `common/src/types.rs` 中的 User 结构体
- [ ] 修改 `database-user/src/lib.rs` 中的 DbUser 结构体
- [ ] 添加 BalanceLog 结构体

#### Phase 3: 充值系统改造
- [ ] 修改 TopupDto 支持货币类型
- [ ] 修改 create_recharge 函数
- [ ] 实现 update_balance 双币版本

#### Phase 4: 扣费系统改造
- [ ] 实现 deduct_dual_currency 函数
- [ ] 实现 check_balance_sufficient 函数
- [ ] 修改 router/src/lib.rs 扣费调用点

#### Phase 5: 价格选择集成
- [ ] 实现 select_price_for_region 函数
- [ ] 集成 prices_v2 表查询

### 验证方案

#### 单元测试
```rust
#[test]
fn test_dual_currency_deduction() {
    // 1. 用户有 10 USD 和 100 CNY
    // 2. 消费美国模型 $5
    //    → 期望：balance_usd = 5
    // 3. 消费中国模型 ¥30
    //    → 期望：balance_cny = 70
    // 4. 消费美国模型 $10（USD 不足）
    //    → 期望：balance_usd = 0, balance_cny = 64 (汇率 7.2)
}

#[test]
fn test_insufficient_balance() {
    // 用户余额不足时返回错误
}

#[test]
fn test_exchange_rate_conversion() {
    // 汇率转换正确性
}
```

#### 集成测试
- [ ] 充值 USD → 消费美国模型
- [ ] 充值 CNY → 消费中国模型
- [ ] 充值 USD → 消费中国模型（跨币种）
- [ ] 充值 CNY → 消费美国模型（跨币种）
- [ ] 混合余额扣费测试

#### 手动验证
- [ ] 充值页面选择货币
- [ ] 余额显示正确
- [ ] 账单/流水按 preferred_currency 显示

### 相关代码位置

- 用户余额 CRUD: `crates/database/crates/database-user/src/lib.rs`
- Token 配额扣减: `crates/database/crates/database-router/src/lib.rs`
- 计费核心算法: `crates/router/src/billing.rs`
- 价格同步服务: `crates/router/src/price_sync.rs`
- 汇率服务: `crates/router/src/exchange_rate.rs`
- Router 扣费流程: `crates/router/src/lib.rs`
- 数据库 Schema: `crates/database/src/schema.rs`
- 数据类型定义: `crates/common/src/types.rs`

---
十七、价格系统 u64 精度迁移

### 问题背景

当前 BurnCloud 的价格系统使用 `f64` 类型存储所有价格字段，存在以下问题：
- 浮点精度问题：累加计算时可能产生微小误差
- 比较问题：直接比较浮点数不可靠
- 审计问题：财务计算应使用精确数值

**解决方案**: 将所有价格字段从 `f64` 迁移到 `u64`，使用纳美元 (Nanodollars) 作为内部存储单位。

### 存储单位选择

**使用：纳美元 (Nanodollars)** - 提供 9 位小数精度

- 1 USD = 1,000,000,000 纳美元 (10^9)
- 9 位小数精度，可表示 $0.000000001
- u64 范围：最大可表示 ~$18.4 十亿美元（单价）
- 对于总价计算，使用 u128 或分步计算避免溢出

| 当前价格 (f64) | u64 纳美元 |
|---------------|-----------|
| $3.0/1M tokens | 3,000,000,000 |
| $0.15/1M tokens | 150,000,000 |
| $0.00015/token | 150,000 |
| $30.0/1M tokens | 30,000,000,000 |

### 精度对比

| 单位 | 精度 | 最大美元值 (u64) |
|------|------|-----------------|
| 美元 | 0 | $18,446,744,073,709,551,615 |
| 微美元 (10^-6) | 6 位 | $18,446,744,073,709 |
| **纳美元 (10^-9)** | **9 位** | **$18,446,744,073** |
| 皮美元 (10^-12) | 12 位 | $18,446,744 |

### 辅助模块设计

**文件: `crates/common/src/price_u64.rs`** (新建)

```rust
/// 纳美元常量：1 USD = 1,000,000,000 纳美元
pub const NANO_PER_DOLLAR: u64 = 1_000_000_000;

/// 汇率缩放因子（9 位小数精度）
pub const RATE_SCALE: u64 = 1_000_000_000;

/// f64 美元价格 → u64 纳美元
pub fn dollars_to_nano(price: f64) -> u64 {
    (price * NANO_PER_DOLLAR as f64).round() as u64
}

/// u64 纳美元 → f64 美元价格
pub fn nano_to_dollars(nano: u64) -> f64 {
    nano as f64 / NANO_PER_DOLLAR as f64
}

/// f64 汇率 → u64 缩放汇率
pub fn rate_to_scaled(rate: f64) -> u64 {
    (rate * RATE_SCALE as f64).round() as u64
}

/// u64 缩放汇率 → f64 汇率
pub fn scaled_to_rate(scaled: u64) -> f64 {
    scaled as f64 / RATE_SCALE as f64
}

/// 安全的价格乘法（避免溢出）：tokens * price_per_million / 1_000_000
pub fn calculate_cost_safe(tokens: u64, price_per_million_nano: u64) -> u64 {
    // 使用 u128 中间结果避免溢出
    let result = (tokens as u128) * (price_per_million_nano as u128) / 1_000_000;
    result as u64
}
```

### 涉及的文件修改

| 文件 | 修改内容 |
|------|----------|
| `crates/common/src/price_u64.rs` | 新建，转换函数 |
| `crates/common/src/lib.rs` | 导出新模块 |
| `crates/common/src/types.rs` | PriceV2, TieredPrice, ExchangeRate 结构体 f64→u64 |
| `crates/common/src/pricing_config.rs` | CurrencyPricing, TieredPriceConfig 等 f64→u64 |
| `crates/database/src/schema.rs` | 数据库表定义 REAL→BIGINT |
| `crates/database-models/src/lib.rs` | Price, TieredPrice, PriceV2 模型 f64→u64 |
| `crates/router/src/billing.rs` | 计费计算函数 u64 运算 |
| `crates/router/src/exchange_rate.rs` | 汇率服务 u64 |
| `crates/router/src/price_sync.rs` | 价格同步边界转换 |
| `crates/cli/src/price.rs` | CLI 输入/输出 f64，内部 u64 |
| `crates/cli/src/currency.rs` | 汇率 CLI u64 |

### 实施步骤

#### Phase 1: 添加基础设施
- [ ] 创建 `price_u64.rs` 模块
- [ ] 添加转换函数和常量
- [ ] 在 `common/Cargo.toml` 中导出

#### Phase 2: 修改类型定义
- [ ] 修改 `types.rs` 中的结构体
- [ ] 修改 `pricing_config.rs` 中的结构体
- [ ] 添加自定义 serde 序列化器保持 JSON 兼容

#### Phase 3: 修改数据库层
- [ ] 修改 `schema.rs` 添加 BIGINT 列
- [ ] 添加数据迁移脚本
- [ ] 修改 `database-models/lib.rs`

#### Phase 4: 修改业务逻辑
- [ ] 修改 `billing.rs` 计费计算
- [ ] 修改 `exchange_rate.rs` 汇率服务
- [ ] 修改 `price_sync.rs` 价格同步

#### Phase 5: 修改 CLI
- [ ] 修改 `price.rs` 命令处理
- [ ] 修改 `currency.rs` 命令处理

#### Phase 6: 测试和验证
- [ ] 更新所有单元测试
- [ ] 运行集成测试
- [ ] 手动验证 CLI 功能

### 数据库迁移策略

#### SQLite 迁移脚本
```sql
-- 1. 创建新表（使用 BIGINT）
CREATE TABLE prices_v2_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model TEXT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    input_price BIGINT NOT NULL DEFAULT 0,
    output_price BIGINT NOT NULL DEFAULT 0,
    cache_read_input_price BIGINT,
    cache_creation_input_price BIGINT,
    batch_input_price BIGINT,
    batch_output_price BIGINT,
    priority_input_price BIGINT,
    priority_output_price BIGINT,
    audio_input_price BIGINT,
    source TEXT,
    region TEXT,
    context_window BIGINT,
    max_output_tokens BIGINT,
    supports_vision INTEGER DEFAULT 0,
    supports_function_calling INTEGER DEFAULT 0,
    synced_at BIGINT,
    created_at BIGINT,
    updated_at BIGINT,
    UNIQUE(model, currency, region)
);

-- 2. 迁移数据（REAL * 1,000,000,000 → BIGINT）
INSERT INTO prices_v2_new SELECT
    id, model, currency,
    CAST(ROUND(input_price * 1000000000) AS BIGINT),
    CAST(ROUND(output_price * 1000000000) AS BIGINT),
    -- ... 其他字段
FROM prices_v2;

-- 3. 切换表
DROP TABLE prices_v2;
ALTER TABLE prices_v2_new RENAME TO prices_v2;
```

#### PostgreSQL 迁移脚本
```sql
ALTER TABLE prices_v2
    ALTER COLUMN input_price TYPE BIGINT USING ROUND(input_price * 1000000000)::BIGINT,
    ALTER COLUMN output_price TYPE BIGINT USING ROUND(output_price * 1000000000)::BIGINT;
-- ... 其他字段
```

### 验证方案

#### 单元测试
```rust
#[test]
fn test_dollars_to_nano_roundtrip() {
    let prices = [3.0, 0.15, 30.0, 0.00015, 0.000000001];
    for price in prices {
        let nano = dollars_to_nano(price);
        let back = nano_to_dollars(nano);
        assert!((price - back).abs() < 0.000000001);
    }
}

#[test]
fn test_nine_decimal_precision() {
    assert_eq!(dollars_to_nano(0.000000001), 1);
    assert_eq!(dollars_to_nano(1.000000001), 1_000_000_001);
}
```

#### 手动验证
```bash
# 设置价格并验证存储
burncloud price set gpt-4o --input 2.5 --output 10.0 --currency USD
sqlite3 test.db "SELECT input_price FROM prices_v2 WHERE model='gpt-4o';"
# 期望: 2500000000

# 验证 9 位精度
burncloud price set test-model --input 0.000000123 --output 0.000000456
sqlite3 test.db "SELECT input_price FROM prices_v2 WHERE model='test-model';"
# 期望: 123
```

### 风险和缓解

| 风险 | 缓解措施 |
|------|---------|
| 精度丢失 | 纳美元提供 9 位小数，足够 token 定价 |
| 溢出 | 单价 u64，总价计算用 u128 中间值 |
| API 兼容性 | JSON 序列化输出浮点格式 |
| LiteLLM 同步 | 边界处转换 f64→u64 |
| CLI 输入 | 用户输入 f64，内部转换存储 |
| 数据迁移 | 备份数据库，迁移后验证对比 |

---
十八、区域定价与双币扣费

### 问题描述
1. 国内模型（如 Qwen）有人民币价格，但系统可能把它们硬转成美元
2. 当汇率变动时，硬转换的价格就不准确了
3. `docs/config/pricing.example.json` 中的设计错误：同一个模型同时有 USD 和 CNY 两种价格
4. 旧的 `quota` 字段已废弃，需要移除
5. 旧的 `prices` 表应被 `prices_v2` 正式替换

### 核心原则
**同一个渠道同一个模型只能有一种货币定价**
- 国内渠道 → 使用人民币定价（CNY）
- 海外渠道 → 使用美元定价（USD）
- **绝对不能把人民币价格硬转成美元**

### 设计目标
- **prices_v2 正式替换 prices**：移除旧表，使用 nanodollars 精度
- **修改约束**：`UNIQUE(model, region)` 确保一个区域只有一种货币价格
- **移除 quota**：使用双币钱包替代
- **双币扣费**：根据模型区域优先扣对应币种

### 价格表变更

**废弃旧表 `prices`**，正式使用 `prices_v2`：

```sql
-- prices_v2 表结构（nanodollars 精度）
CREATE TABLE prices_v2 (
    model TEXT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    input_price BIGINT NOT NULL DEFAULT 0,    -- nanodollars (i64)
    output_price BIGINT NOT NULL DEFAULT 0,
    region TEXT,                               -- 'cn', 'international', NULL
    -- 高级定价字段...
    UNIQUE(model, region)  -- 修改约束：一个区域只有一种货币
);
```

**约束变更**：`UNIQUE(model, currency, region)` → `UNIQUE(model, region)`

```sql
-- 正确：每个区域只有一条记录
(qwen-max, CNY, cn, ¥0.359)           -- 国内用人民币
(qwen-max, USD, international, $1.2)  -- 海外用美元
```

### 扣费逻辑

**请求前预判**（基于 max_tokens）：
```
1. 从请求体提取 input_tokens（估算）和 max_tokens（用户指定或模型默认）
2. 计算最大预估费用 = (input + max_output) × 价格
3. 检查余额是否充足
4. 余额不足 → 直接拒绝，返回 402 错误
```

**请求后扣费**：
```
美国模型 (pricing_region = "international", 价格是 USD):
  → 计算费用（USD 原价，无汇率转换）
  → 优先扣 USD 余额
  → USD 不足时，CNY 按实时汇率转换补足

中国模型 (pricing_region = "cn", 价格是 CNY):
  → 计算费用（CNY 原价，无汇率转换）
  → 优先扣 CNY 余额
  → CNY 不足时，USD 按实时汇率转换补足
```

### 实现步骤

#### Step 1: 数据库迁移 - 移除 quota，添加双币钱包

**文件**: `crates/database/src/schema.rs`

```sql
-- 移除 quota 相关字段
ALTER TABLE users DROP COLUMN quota;
ALTER TABLE users DROP COLUMN used_quota;

-- 添加双币钱包（nanodollars 精度，使用 UNSIGNED 保证非负）
ALTER TABLE users ADD COLUMN balance_usd BIGINT UNSIGNED DEFAULT 0;
ALTER TABLE users ADD COLUMN balance_cny BIGINT UNSIGNED DEFAULT 0;
ALTER TABLE users ADD COLUMN preferred_currency VARCHAR(10) DEFAULT 'USD';

-- 移除 router_tokens 表的 quota 字段
ALTER TABLE router_tokens DROP COLUMN quota_limit;
ALTER TABLE router_tokens DROP COLUMN used_quota;
ALTER TABLE router_tokens DROP COLUMN unlimited_quota;
```

**设计决策**：余额使用 **u64**（无符号 64 位整数）
- 余额物理上不可能为负，类型系统强制约束
- 扣费前必须检查余额充足性
- 与 `price_u64.rs` 价格设计一致

#### Step 2: prices_v2 替换 prices 表

**文件**: `crates/database/src/schema.rs`

```sql
-- 修改唯一约束
-- 当前（错误）: UNIQUE(model, currency, region)
-- 修改为（正确）: UNIQUE(model, region)

-- 废弃旧 prices 表
ALTER TABLE prices RENAME TO prices_deprecated;
```

**文件**: `crates/database/crates/database-models/src/lib.rs`

```rust
// 移除 PriceModel（旧）
// 使用 PriceV2Model 作为唯一价格接口

// PriceV2Model::get_by_model_region（简化查询）
pub async fn get_by_model_region(
    db: &Database,
    model: &str,
    region: Option<&str>,
) -> Result<Option<PriceV2>> {
    // 直接查询 (model, region)
}

// PriceV2Model::upsert（修改约束）
ON CONFLICT(model, region) DO UPDATE SET ...
```

#### Step 3: 更新类型定义

**文件**: `crates/common/src/types.rs`

```rust
pub struct User {
    // 移除
    // pub quota: i64,
    // pub used_quota: i64,

    // 新增双币钱包（u64 保证非负）
    pub balance_usd: u64,        // USD 余额 (nanodollars)
    pub balance_cny: u64,        // CNY 余额 (nanodollars)
    pub preferred_currency: Option<String>,
}

pub struct Token {
    // 移除
    // pub quota_limit: i64,
    // pub used_quota: i64,
    // pub unlimited_quota: bool,
}
```

#### Step 4: 实现双币扣费逻辑

**文件**: `crates/database/crates/database-router/src/lib.rs`

```rust
/// 双币扣费核心逻辑
pub async fn deduct_dual_currency(
    db: &Database,
    user_id: &str,
    cost_nano: u64,           // 费用 (nanodollars)
    cost_currency: Currency,  // 费用币种
    exchange_rate: i64,       // 汇率 (scaled by 10^9)
) -> Result<DeductResult> {
    let user = UserModel::get(db, user_id).await?;

    match cost_currency {
        Currency::CNY => {
            // 中国模型：优先扣 CNY
            if user.balance_cny >= cost_nano {
                deduct_cny(db, user_id, cost_nano).await
            } else {
                // CNY 不足，用 USD 补足
                let shortfall = cost_nano - user.balance_cny;
                let usd_needed = (shortfall as u128 * 1_000_000_000 / exchange_rate as u128) as u64;
                deduct_cny(db, user_id, user.balance_cny).await?;
                deduct_usd(db, user_id, usd_needed).await
            }
        }
        Currency::USD => {
            // 美国模型：优先扣 USD
            if user.balance_usd >= cost_nano {
                deduct_usd(db, user_id, cost_nano).await
            } else {
                // USD 不足，用 CNY 补足
                let shortfall = cost_nano - user.balance_usd;
                let cny_needed = (shortfall as u128 * exchange_rate as u128 / 1_000_000_000) as u64;
                deduct_usd(db, user_id, user.balance_usd).await?;
                deduct_cny(db, user_id, cny_needed).await
            }
        }
        _ => Err(anyhow!("Unsupported currency")),
    }
}
```

#### Step 5: 路由层集成

**文件**: `crates/router/src/lib.rs`

##### 5.1 修改 proxy_logic 返回 pricing_region
```rust
async fn proxy_logic(...) -> (Response, Option<String>, StatusCode, Option<String>)
//                                                      ^^^^^^^^^^^^ pricing_region
```

##### 5.2 获取区域对应的价格
```rust
let price = PriceV2Model::get_by_model_region(
    &state.db,
    model,
    pricing_region.as_deref(),
).await?;

let cost_currency = price.currency;  // "USD" 或 "CNY"
```

##### 5.3 请求前余额预判（基于 max_tokens）
```rust
// 从请求体提取 max_tokens
let max_output_tokens = body_json.get("max_tokens")
    .and_then(|v| v.as_u64())
    .unwrap_or(price.max_output_tokens.unwrap_or(4096));

// 计算最大预估费用
let estimated_input_tokens = (body_bytes.len() as f32 / 4.0).ceil() as u64;
let max_cost_nano = billing::calculate_cost_nano(
    estimated_input_tokens,
    max_output_tokens,
    &price,
);

// 检查余额（考虑双币）
let (primary_balance, secondary_balance, rate) = match cost_currency {
    Currency::CNY => (user.balance_cny, user.balance_usd, exchange_rate),
    _ => (user.balance_usd, user.balance_cny, 1_000_000_000 / exchange_rate),
};

let total_available = primary_balance + (secondary_balance as f64 * rate as f64 / 1e9) as u64;
if total_available < max_cost_nano {
    return Response::builder()
        .status(402)
        .body(Body::from(r#"{"error":{"message":"Insufficient balance","type":"insufficient_balance"}}"#))
        .unwrap();
}
```

##### 5.4 请求后调用双币扣费
```rust
let exchange_rate = state.exchange_rate_service.get_rate_nano(Currency::USD, Currency::CNY)
    .unwrap_or(7_200_000_000);  // 7.2 * 10^9

RouterDatabase::deduct_dual_currency(
    &state.db,
    &user_id,
    cost_nano,
    cost_currency,
    exchange_rate,
).await?;
```

#### Step 6: 修正价格配置示例

**文件**: `docs/config/pricing.example.json`

```json
{
  "version": "2.0",
  "models": {
    "qwen-max": [
      {
        "region": "cn",
        "currency": "CNY",
        "input_price": 0.359,
        "output_price": 1.434
      },
      {
        "region": "international",
        "currency": "USD",
        "input_price": 1.2,
        "output_price": 6.0
      }
    ]
  }
}
```

### 关键文件清单

| 文件 | 改动 |
|------|------|
| `crates/database/src/schema.rs` | 移除 quota；prices_v2 约束改为 UNIQUE(model, region)；废弃 prices 表 |
| `crates/database/crates/database-models/src/lib.rs` | 移除 PriceModel；PriceV2Model::get_by_model_region |
| `crates/common/src/types.rs` | User 移除 quota，添加 balance_usd/balance_cny |
| `crates/database/crates/database-router/src/lib.rs` | 移除 deduct_quota，添加 deduct_dual_currency |
| `crates/router/src/lib.rs` | 使用 prices_v2；调用双币扣费；余额预判 |
| `docs/config/pricing.example.json` | 修正为每个区域只有一种货币 |

### 验证方案

#### 单元测试
```rust
#[test]
fn test_one_currency_per_region() {
    // 尝试为同一模型+区域插入两种货币价格 → 应该失败
}

#[test]
fn test_balance_check_with_max_tokens() {
    // 用户余额 10 CNY，请求 max_tokens=10000
    // 预估费用 > 10 CNY → 应该拒绝
}

#[test]
fn test_dual_currency_cn_model() {
    // 消费 cn 区域模型（CNY 定价）
    // → 期望：balance_cny 减少
}

#[test]
fn test_cross_currency_deduction() {
    // CNY 不足时用 USD 补足
}
```

#### 集成测试
```bash
# 设置国内模型价格（只有 CNY）
burncloud price set qwen-max --input 0.359 --output 1.434 --currency CNY --region cn

# 验证无法再设置 USD 价格（同一 region）
burncloud price set qwen-max --input 1.2 --output 6.0 --currency USD --region cn
# 期望：错误

# 设置海外价格（只有 USD）
burncloud price set qwen-max --input 1.2 --output 6.0 --currency USD --region international
```

### 性能影响

**max_tokens 预判**增加约 1-5ms（一次数据库查询），相比上游 API 调用（100ms-30s）可忽略不计。
