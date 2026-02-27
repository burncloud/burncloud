# 统一 Usage 解析模块设计方案

> **目标**: 设计一个统一的 LLM Usage 解析模块，支持 OpenAI、Anthropic、Gemini 等多提供商，精确计算所有类型 token 的费用。

---

## 0. 命名规范 (Naming Conventions)

> **重要**: 所有数据库表、结构体、类型命名**禁止使用版本号后缀** (v2, v3 等)

| 类型 | ✅ 正确 | 🛑 禁止 |
|------|--------|--------|
| 数据库表 | `prices`, `router_logs` | ~~`prices_v3`~~, ~~`router_logs_v2`~~ |
| Rust 结构体 | `DbRouterLog`, `Price` | ~~`DbRouterLogV2`~~, ~~`PriceV3`~~ |
| SQL 索引 | `idx_router_logs_model` | ~~`idx_router_logs_v2_model`~~ |

**迁移策略**: 直接修改原有表结构 (`ALTER TABLE`)，不创建新版本表。

---

## 1. 模块结构

```
crates/router/src/usage/
├── mod.rs              # 模块入口，统一接口
├── types.rs            # 核心类型定义
├── parser.rs           # Usage 解析器 trait 和工厂
├── providers/
│   ├── mod.rs
│   ├── openai.rs       # OpenAI 格式解析
│   ├── anthropic.rs    # Anthropic 格式解析
│   ├── gemini.rs       # Gemini 格式解析
│   ├── deepseek.rs     # DeepSeek 格式解析
│   └── generic.rs      # 通用/未知格式解析
├── calculator.rs       # 费用计算器
└── tests.rs            # 集成测试
```

---

## 2. 核心类型定义 (`types.rs`)

### 2.1 统一的 Token 使用量结构

```rust
/// 统一的 Token 使用量结构
/// 支持所有主流 LLM 提供商的 token 类型
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnifiedUsage {
    // ========================================
    // 基础 Token (所有模型都有)
    // ========================================
    /// 普通输入 token (不含缓存)
    pub input_tokens: i64,
    /// 输出 token
    pub output_tokens: i64,

    // ========================================
    // 缓存 Token (Prompt Caching)
    // ========================================
    /// 缓存命中 token (90% 折扣)
    pub cache_read_tokens: i64,
    /// 缓存写入 token (1.25x 价格)
    pub cache_write_tokens: i64,

    // ========================================
    // 多模态 Token
    // ========================================
    /// 音频输入 token (GPT-4o-audio, Gemini)
    pub audio_input_tokens: i64,
    /// 音频输出 token (GPT-4o-audio)
    pub audio_output_tokens: i64,
    /// 图像 token (GPT-4V, Claude Vision, Gemini)
    pub image_tokens: i64,
    /// 视频 token (Gemini 1.5 Pro)
    pub video_tokens: i64,

    // ========================================
    // 推理 Token (o1/o3/DeepSeek-R1)
    // ========================================
    /// 思维链 token
    pub reasoning_tokens: i64,
    /// 接受的预测 token (o1)
    pub accepted_prediction_tokens: i64,
    /// 拒绝的预测 token (o1)
    pub rejected_prediction_tokens: i64,

    // ========================================
    // Embedding Token (text-embedding-3 等)
    // ========================================
    /// 向量化 token (仅输入，无输出)
    pub embedding_tokens: i64,

    // ========================================
    // 元数据
    // ========================================
    /// 请求模式
    pub request_mode: RequestMode,
}

/// 请求模式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RequestMode {
    #[default]
    Standard,       // 标准请求
    Batch,          // 批处理 (50% 折扣)
    Priority,       // 高优先级 (170% 价格)
    Flex,           // 灵活/低优先级
}
```

### 2.2 费用明细结构

```rust
/// 费用明细 (所有金额为纳美元 i64)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostBreakdown {
    // ========================================
    // 输入费用
    // ========================================
    /// 普通输入费用
    pub input_cost: i64,
    /// 缓存读取费用 (90% 折扣)
    pub cache_read_cost: i64,
    /// 缓存写入费用 (1.25x)
    pub cache_write_cost: i64,
    /// 音频输入费用
    pub audio_input_cost: i64,
    /// 图像费用
    pub image_cost: i64,
    /// 视频费用
    pub video_cost: i64,
    /// Embedding 费用
    pub embedding_cost: i64,

    // ========================================
    // 输出费用
    // ========================================
    /// 普通输出费用
    pub output_cost: i64,
    /// 音频输出费用
    pub audio_output_cost: i64,
    /// 推理费用
    pub reasoning_cost: i64,

    // ========================================
    // 总计
    // ========================================
    /// 总费用 (纳美元)
    pub total_cost: i64,

    // ========================================
    // 元数据
    // ========================================
    /// 计费货币
    pub currency: Currency,
    /// 使用的价格配置来源
    pub price_source: PriceSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PriceSource {
    #[default]
    Database,       // 数据库 prices 表
    Default,        // 默认价格 (未配置时)
    Tiered,         // 阶梯价格
}
```

### 2.3 原始 API 响应结构

```rust
/// OpenAI API usage 格式
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: Option<i64>,
    pub prompt_tokens_details: Option<OpenAIPromptDetails>,
    pub completion_tokens_details: Option<OpenAICompletionDetails>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIPromptDetails {
    pub cached_tokens: Option<i64>,
    pub audio_tokens: Option<i64>,
    pub image_tokens: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAICompletionDetails {
    pub reasoning_tokens: Option<i64>,
    pub audio_tokens: Option<i64>,
    pub accepted_prediction_tokens: Option<i64>,
    pub rejected_prediction_tokens: Option<i64>,
}

/// Anthropic API usage 格式
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_input_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
    // Claude 4 支持扩展 thinking
    pub cache_creation: Option<AnthropicCacheCreation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicCacheCreation {
    pub ephemeral_5m_input_tokens: Option<i64>,
    pub ephemeral_1h_input_tokens: Option<i64>,
}

/// Gemini API usage 格式
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiUsage {
    pub prompt_token_count: i64,
    pub candidates_token_count: i64,
    pub total_token_count: Option<i64>,
    pub cached_content_token_count: Option<i64>,
    // 多模态
    pub audio_tokens_count: Option<i64>,
    pub image_tokens_count: Option<i64>,
    pub video_tokens_count: Option<i64>,
}

/// DeepSeek API usage 格式 (兼容 OpenAI + 推理扩展)
#[derive(Debug, Clone, Deserialize)]
pub struct DeepSeekUsage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: Option<i64>,
    pub prompt_cache_hit_tokens: Option<i64>,       // 缓存命中
    pub prompt_cache_miss_tokens: Option<i64>,      // 缓存未命中
    pub reasoning_tokens: Option<i64>,              // 思维链
}
```

---

## 3. 解析器 Trait (`parser.rs`)

```rust
/// Usage 解析器 Trait
/// 不同提供商实现此 trait
pub trait UsageParser: Send + Sync {
    /// 从原始 JSON 响应解析 usage
    fn parse(&self, json: &Value) -> Result<UnifiedUsage>;

    /// 从流式响应的 message_start/delta 事件解析 usage
    fn parse_streaming(&self, event_data: &Value) -> Result<UnifiedUsage>;

    /// 支持的提供商标识
    fn provider(&self) -> &'static str;
}

/// 解析器工厂
pub struct UsageParserFactory;

impl UsageParserFactory {
    /// 根据协议类型获取解析器
    pub fn get_parser(protocol: &str) -> Box<dyn UsageParser> {
        match protocol.to_lowercase().as_str() {
            "openai" | "azure" => Box::new(OpenAIUsageParser),
            "anthropic" | "claude" => Box::new(AnthropicUsageParser),
            "gemini" | "vertex" => Box::new(GeminiUsageParser),
            "deepseek" => Box::new(DeepSeekUsageParser),
            _ => Box::new(GenericUsageParser),
        }
    }

    /// 从响应 body 自动检测协议并解析
    pub fn auto_parse(json: &Value) -> Result<(String, UnifiedUsage)> {
        // 检测逻辑...
    }
}
```

---

## 4. 各提供商解析器字段映射

### 4.1 字段映射表

| UnifiedUsage 字段 | OpenAI | Anthropic | Gemini | DeepSeek |
|-------------------|--------|-----------|--------|----------|
| **基础** |||||
| input_tokens | prompt_tokens | input_tokens | prompt_token_count | prompt_tokens |
| output_tokens | completion_tokens | output_tokens | candidates_token_count | completion_tokens |
| **缓存** |||||
| cache_read_tokens | prompt_tokens_details.cached_tokens | cache_read_input_tokens | cached_content_token_count | prompt_cache_hit_tokens |
| cache_write_tokens | (无) | cache_creation_input_tokens | (无) | (无) |
| **多模态** |||||
| audio_input_tokens | prompt_tokens_details.audio_tokens | (无) | audio_tokens_count | (无) |
| audio_output_tokens | completion_tokens_details.audio_tokens | (无) | (无) | (无) |
| image_tokens | prompt_tokens_details.image_tokens | (无) | image_tokens_count | (无) |
| video_tokens | (无) | (无) | video_tokens_count | (无) |
| **推理** |||||
| reasoning_tokens | completion_tokens_details.reasoning_tokens | (无) | (无) | reasoning_tokens |
| accepted_prediction_tokens | completion_tokens_details.accepted_prediction_tokens | (无) | (无) | (无) |
| rejected_prediction_tokens | completion_tokens_details.rejected_prediction_tokens | (无) | (无) | (无) |
| **Embedding** |||||
| embedding_tokens | prompt_tokens (embedding 模型) | (无) | prompt_token_count | (无) |

### 4.2 解析示例

**OpenAI 响应:**
```json
{
  "usage": {
    "prompt_tokens": 1117,
    "completion_tokens": 46,
    "total_tokens": 1163,
    "prompt_tokens_details": {
      "cached_tokens": 0,
      "audio_tokens": 0
    },
    "completion_tokens_details": {
      "reasoning_tokens": 0,
      "audio_tokens": 0,
      "accepted_prediction_tokens": 0,
      "rejected_prediction_tokens": 0
    }
  }
}
```

**Anthropic 响应:**
```json
{
  "usage": {
    "input_tokens": 100,
    "output_tokens": 50,
    "cache_creation_input_tokens": 2000,
    "cache_read_input_tokens": 0
  }
}
```

**Gemini 响应:**
```json
{
  "usageMetadata": {
    "promptTokenCount": 1000,
    "candidatesTokenCount": 200,
    "totalTokenCount": 1200,
    "cachedContentTokenCount": 500
  }
}
```

---

## 5. 费用计算器 (`calculator.rs`)

### 5.1 计算逻辑

```rust
/// 费用计算器
pub struct CostCalculator {
    /// 价格配置
    price: Price,
}

impl CostCalculator {
    /// 计算 usage 对应的费用
    pub fn calculate(&self, usage: &UnifiedUsage) -> CostBreakdown {
        let mut breakdown = CostBreakdown::default();

        // 1. 输入费用
        breakdown.input_cost = self.calc_input_cost(usage);
        breakdown.cache_read_cost = self.calc_cache_read_cost(usage);
        breakdown.cache_write_cost = self.calc_cache_write_cost(usage);

        // 2. 输出费用
        breakdown.output_cost = self.calc_output_cost(usage);

        // 3. 多模态费用
        breakdown.audio_input_cost = self.calc_audio_input_cost(usage);
        breakdown.audio_output_cost = self.calc_audio_output_cost(usage);
        breakdown.image_cost = self.calc_image_cost(usage);
        breakdown.video_cost = self.calc_video_cost(usage);

        // 4. 推理费用
        breakdown.reasoning_cost = self.calc_reasoning_cost(usage);

        // 5. Embedding 费用
        breakdown.embedding_cost = self.calc_embedding_cost(usage);

        // 6. 总计
        breakdown.total_cost = breakdown.input_cost
            + breakdown.cache_read_cost
            + breakdown.cache_write_cost
            + breakdown.output_cost
            + breakdown.audio_input_cost
            + breakdown.audio_output_cost
            + breakdown.image_cost
            + breakdown.video_cost
            + breakdown.reasoning_cost
            + breakdown.embedding_cost;

        breakdown
    }
}
```

### 5.2 默认价格倍率表

| Token 类型 | 默认倍率 | 说明 |
|------------|----------|------|
| **基础** |||
| Standard Input | 1.0x | 基准价格 |
| Standard Output | 2-3x | 通常为输入的 2-3 倍 |
| **缓存** |||
| Cache Read | 0.1x | 90% 折扣 |
| Cache Write | 1.25x | 25% 额外费用 |
| **批处理** |||
| Batch Input/Output | 0.5x | 50% 折扣 |
| **优先级** |||
| Priority Input/Output | 1.7x | 70% 加价 |
| **多模态** |||
| Audio Input | 7x | 约为文本的 7 倍 |
| Audio Output | 10x+ | 价格较高 |
| Image Tokens | 模型定价 | 按图像大小/分辨率计费 |
| Video Tokens | 模型定价 | 按视频时长/帧数计费 |
| **推理** |||
| Reasoning | 1.0x | 通常与 output 同价 |
| **Embedding** |||
| Embedding Input | 0.01x | 极低价格 |

### 5.3 价格表扩展

> **命名规范**: 直接修改原有 `prices` 表，不使用版本号后缀 (v2, v3)

```sql
-- 扩展后的 prices 表 (直接替换原表)
CREATE TABLE prices (
    id INTEGER PRIMARY KEY,
    model TEXT NOT NULL UNIQUE,

    -- 基础价格 (纳美元/百万token)
    input_price INTEGER NOT NULL,
    output_price INTEGER NOT NULL,

    -- 缓存价格 (可选，NULL则使用默认倍率)
    cache_read_price INTEGER,               -- NULL = input_price * 0.1
    cache_write_price INTEGER,              -- NULL = input_price * 1.25

    -- 批处理价格 (可选)
    batch_input_price INTEGER,              -- NULL = input_price * 0.5
    batch_output_price INTEGER,             -- NULL = output_price * 0.5

    -- 多模态价格 (可选)
    audio_input_price INTEGER,
    audio_output_price INTEGER,
    image_price INTEGER,                    -- 每张图或每 1K 图像 token
    video_price INTEGER,                    -- 每秒视频或每 1K 视频 token

    -- 推理价格 (可选)
    reasoning_price INTEGER,                -- o1 等模型的思维链价格

    -- Embedding 价格
    embedding_price INTEGER,                -- 每 1M token

    -- 元数据
    currency TEXT DEFAULT 'USD',
    pricing_region TEXT,
    effective_date TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

---

## 6. 扩展的日志表结构

> **命名规范**: 直接修改原有 `router_logs` 表，不使用版本号后缀 (v2, v3)

### 6.1 扩展后的 DbRouterLog 结构

```sql
-- 扩展后的 router_logs 表 (直接替换原表)
CREATE TABLE router_logs (
    id INTEGER PRIMARY KEY,
    request_id TEXT NOT NULL,
    user_id TEXT,
    channel_id TEXT,
    model TEXT NOT NULL,

    -- Token 计数 - 基础
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,

    -- Token 计数 - 缓存
    cache_read_tokens INTEGER DEFAULT 0,
    cache_write_tokens INTEGER DEFAULT 0,

    -- Token 计数 - 多模态
    audio_input_tokens INTEGER DEFAULT 0,
    audio_output_tokens INTEGER DEFAULT 0,
    image_tokens INTEGER DEFAULT 0,
    video_tokens INTEGER DEFAULT 0,

    -- Token 计数 - 推理
    reasoning_tokens INTEGER DEFAULT 0,

    -- Token 计数 - Embedding
    embedding_tokens INTEGER DEFAULT 0,

    -- 费用 (纳美元)
    input_cost INTEGER DEFAULT 0,
    output_cost INTEGER DEFAULT 0,
    cache_cost INTEGER DEFAULT 0,
    audio_cost INTEGER DEFAULT 0,
    image_cost INTEGER DEFAULT 0,
    video_cost INTEGER DEFAULT 0,
    reasoning_cost INTEGER DEFAULT 0,
    embedding_cost INTEGER DEFAULT 0,
    total_cost INTEGER DEFAULT 0,

    -- 请求模式
    request_mode TEXT DEFAULT 'standard',

    -- 元数据
    latency_ms INTEGER,
    status_code INTEGER,
    is_stream INTEGER DEFAULT 0,
    api_version TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,

    -- 扩展字段 (JSON)
    extra TEXT
);

-- 索引
CREATE INDEX idx_router_logs_model ON router_logs(model);
CREATE INDEX idx_router_logs_user ON router_logs(user_id);
CREATE INDEX idx_router_logs_created ON router_logs(created_at);
```

### 6.2 对应 Rust 结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbRouterLog {
    pub id: i64,
    pub request_id: String,
    pub user_id: Option<String>,
    pub channel_id: Option<String>,
    pub model: String,

    // Token 计数 - 基础
    pub input_tokens: i64,
    pub output_tokens: i64,

    // Token 计数 - 缓存
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,

    // Token 计数 - 多模态
    pub audio_input_tokens: i64,
    pub audio_output_tokens: i64,
    pub image_tokens: i64,
    pub video_tokens: i64,

    // Token 计数 - 推理
    pub reasoning_tokens: i64,

    // Token 计数 - Embedding
    pub embedding_tokens: i64,

    // 费用 (纳美元)
    pub input_cost: i64,
    pub output_cost: i64,
    pub cache_cost: i64,
    pub audio_cost: i64,
    pub image_cost: i64,
    pub video_cost: i64,
    pub reasoning_cost: i64,
    pub embedding_cost: i64,
    pub total_cost: i64,

    // 请求模式
    pub request_mode: String,

    // 元数据
    pub latency_ms: i64,
    pub status_code: i32,
    pub is_stream: i32,
    pub api_version: Option<String>,
    pub created_at: Option<String>,
    pub extra: Option<String>,
}
```

---

## 7. CLI 显示格式

### 7.1 标准显示

```
📊 Usage Report - gpt-4o-2024-08-06
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📝 Tokens:
   ├─ Input:            1,117
   ├─ Output:              46
   ├─ Cache Read:           0  (90% off)
   └─ Cache Write:          0

💰 Cost:
   ├─ Input:          $0.002792
   ├─ Output:         $0.000460
   ├─ Cache:          $0.000000
   └─ ━━━━━━━━━━━━━━━━━━━━━━━━
   └─ Total:          $0.003252

⏱️  Latency: 1,234 ms
```

### 7.2 详细显示 (含多模态)

```
📊 Usage Report - gpt-4o-audio-preview
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📝 Tokens:
   ├─ Input:              500
   ├─ Output:             200
   ├─ Audio Input:       2048  🎵
   ├─ Audio Output:      1024  🎵
   ├─ Cache Read:          0
   └─ Cache Write:      1000

💰 Cost:
   ├─ Input:          $0.001250
   ├─ Output:         $0.002000
   ├─ Audio Input:    $0.014336  🎵
   ├─ Audio Output:   $0.010240  🎵
   ├─ Cache Read:     $0.000000
   ├─ Cache Write:    $0.001250
   └─ ━━━━━━━━━━━━━━━━━━━━━━━━
   └─ Total:          $0.029076

⏱️  Latency: 2,456 ms
```

### 7.3 推理模型显示

```
📊 Usage Report - o1-preview
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📝 Tokens:
   ├─ Input:              500
   ├─ Output:           1,200
   ├─ Reasoning:        5,000  🧠
   └─ Accepted Pred:      120  🎯

💰 Cost:
   ├─ Input:          $0.007500
   ├─ Output:         $0.018000
   ├─ Reasoning:      $0.075000  🧠
   └─ ━━━━━━━━━━━━━━━━━━━━━━━━
   └─ Total:          $0.100500

⏱️  Latency: 15,234 ms
```

### 7.4 Gemini 多模态显示

```
📊 Usage Report - gemini-2.5-pro
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📝 Tokens:
   ├─ Input:            1,000
   ├─ Output:             500
   ├─ Image Tokens:     2,048  🖼️
   ├─ Video Tokens:     5,120  🎬
   └─ Cache Read:         500

💰 Cost:
   ├─ Input:          $0.001250
   ├─ Output:         $0.005000
   ├─ Image:          $0.005120  🖼️
   ├─ Video:          $0.012800  🎬
   ├─ Cache Read:     $0.000063
   └─ ━━━━━━━━━━━━━━━━━━━━━━━━
   └─ Total:          $0.024233

⏱️  Latency: 3,456 ms
```

### 7.5 Embedding 模型显示

```
📊 Usage Report - text-embedding-3-large
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📝 Tokens:
   └─ Embedding:      10,000  📊

💰 Cost:
   └─ Embedding:      $0.000130  📊
   └─ ━━━━━━━━━━━━━━━━━━━━━━━━
   └─ Total:          $0.000130

⏱️  Latency: 234 ms
```

---

## 8. 实现优先级

### Phase 1: 核心功能 (P0)
- [ ] 统一类型定义 (`UnifiedUsage`, `CostBreakdown`)
- [ ] OpenAI 解析器
- [ ] Anthropic 解析器
- [ ] 基础费用计算器
- [ ] 数据库迁移脚本

### Phase 2: 多模态支持 (P1)
- [ ] Gemini 解析器 (含 video tokens)
- [ ] DeepSeek 解析器 (含 reasoning)
- [ ] 多模态费用计算
- [ ] CLI 显示优化

### Phase 3: 高级功能 (P2)
- [ ] 阶梯价格支持
- [ ] 多货币支持
- [ ] 价格缓存
- [ ] 使用量聚合统计

---

## 9. 参考文档

- [OpenAI Chat Completions API](https://platform.openai.com/docs/api-reference/chat/object)
- [Anthropic Prompt Caching](https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching)
- [Gemini API Usage Metadata](https://ai.google.dev/api/generate-content#usage-metadata)
- [DeepSeek API](https://platform.deepseek.com/api-docs/)
