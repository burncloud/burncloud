<!-- /autoplan restore point: /home/core/.gstack/projects/burncloud-burncloud/main-autoplan-restore-20260328-055806.md -->
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

---

## 10. CEO Review (autoplan — 2026-03-28)

> 自动生成 · 分支: main · 审查范围: 全文

### 10.1 方案前提核对 (Premise Gate)

| # | 前提 | 验证结果 | 影响 |
|---|------|---------|------|
| P1 | 所有 Phase 1/2 核心实现已完成 | ✅ 已实现于 `crates/service/crates/service-billing/` (56 tests pass) | 无阻塞 |
| P2 | 模块路径与计划一致 (`crates/router/src/usage/`) | ⚠️ **不一致** — 实际位于 `crates/service/crates/service-billing/src/` | 文档需更新 |
| P3 | `router_logs` 表已包含计划的多维度费用列 | ❌ **未完成** — 实际只有聚合 `cost` 列，无 per-type 费用列 | Phase 3 DB 迁移是阻塞项 |
| P4 | `UnifiedUsage` 字段与计划完全对齐 | ⚠️ 缺少 `accepted_prediction_tokens`, `rejected_prediction_tokens`, `request_mode` | 对 o1 预测费用计算有影响 |
| P5 | `CostBreakdown` 字段与计划完全对齐 | ⚠️ 实际使用 `cache_cost`(合并) 和 `audio_cost`+`voice_cost`(命名奇特)，而非计划的分离字段 | 影响 CLI 细粒度展示 |

**决策**: 前提 P3 是最高优先级缺口。继续 Phase 3 之前必须补充 DB 迁移。

---

### 10.2 CEO 10 项审查

#### §1 用户价值与市场定位

方案解决了一个真实痛点: 每个提供商的计费格式不同，开发者需要自己聚合。统一格式意味着换模型不用改代码。Gemini 2.5 thinking tokens、Anthropic extended cache、OpenAI o1 reasoning 全部覆盖。

**判断**: 价值清晰，对 burncloud 作为多模型代理的定位有直接支撑。

#### §2 功能完整性与边界

计划覆盖 5 个提供商 (OpenAI, Anthropic, Gemini, DeepSeek, Generic)，全部已实现。缺口在于:
- `GeminiUsage` struct 计划了 `audio_tokens_count`, `image_tokens_count` 字段，但解析器里尚未提取这两个字段 (只有 promptTokenCount/candidatesTokenCount/thoughtsTokenCount/cachedContent)。
- DeepSeek `prompt_cache_miss_tokens` 计划映射到 `input_tokens`，需验证计算是否正确 (miss = 未命中缓存，应以正常 input 价格计费)。

**判断**: 核心路径完整。多模态 tokens 的 Gemini 提取是 P1 遗漏项，不影响当前主流用例。

#### §3 技术可行性

设计合理。纳美元 `i64` 存储避免了浮点精度问题，用于计费正确。`saturating_add` 防止 overflow。`PriceCache` key `(model, region)` 是正确的多租户设计。

一个技术风险: SQLite 用 `ALTER TABLE` 逐列迁移的方式无法保证事务性 (SQLite 不支持事务内 DDL rollback)。如果迁移中途失败，会有部分列存在的状态。建议迁移时用 "CREATE new table + INSERT SELECT + RENAME" 模式，与 schema.rs:683 已有的 `router_logs_new RENAME` 逻辑保持一致。

**判断**: 技术路径可行，DB 迁移策略需改进。

#### §4 实现与计划的偏差

| 计划 | 实际 | 评估 |
|------|------|------|
| 路径 `crates/router/src/usage/` | `crates/service/crates/service-billing/src/` | 更合理的位置，但文档未同步 |
| `router_logs` per-type 费用列 | 只有聚合 `cost` 列 | 需要 DB 迁移 |
| `UnifiedUsage.request_mode` | 未实现 | Batch/Priority 定价受影响 |
| `CostBreakdown.cache_read_cost` + `cache_write_cost` | 合并为 `cache_cost` | 丢失了写入缓存的单独可见性 |
| `prices` 表有 `cache_write_price` 列 | 已实现 ✅ | 一致 |
| Tiered pricing | 已实现 ✅ | 超前完成 |
| Phase 1 checkboxes 全未勾选 | 核心已实现 | plan.md 勾选状态需更新 |

#### §5 风险与技术债

1. **DB 迁移未实现**: `router_logs` 当前结构无法存储 per-type 费用。`CostBreakdown` 在计算层是完整的，但计算结果写入 DB 时被丢弃成单个 `cost`。这是计费可追溯性的核心缺口。
2. **`price_sync_handler` dead_code 警告**: 该函数在 server 层注册但编译器警告 dead_code，说明 server 代码路径可能没有真正触发。需确认 `AppState.force_sync_tx` 是否在 server init 时正确注入。
3. **Gemini streaming token 计数修复**: 已在本次会话完成。修复前所有 Gemini 流式请求的费用均为 $0。
4. **`completion_tokens` + `thoughtsTokenCount` 修复**: 已完成。

#### §6 优先级与 ROI

Phase 1/2 已完成 (6 checkboxes 全部实现完毕，只是 plan.md 未更新)。现在最高价值的工作是:

1. **DB 迁移** (高 ROI): 补充 `router_logs` per-type token 和费用列，让 `CostBreakdown` 数据不再丢失
2. **勾选 plan.md Phase 1/2** (低成本): 同步文档现实状态
3. **CLI display** (中 ROI): plan 第 7 节的展示格式尚未实现

#### §7 可扩展性

`UsageParser` trait 设计正确，新增提供商只需实现 trait。`PriceCache` 支持 region，多租户友好。流式和非流式处理分离。

潜在扩展点: 当前 `generic.rs` 会尝试同时检测 OpenAI 和 Anthropic 格式，作为 fallback 合理。但如果某个新提供商的响应格式与两者都相似，可能误判。建议 `generic.rs` 加日志记录检测结果。

#### §8 测试覆盖

56 个 unit tests，全部通过。覆盖:
- 所有 5 个提供商的 parse_response / parse_streaming_chunk
- 思维链 tokens、缓存、SSE prefix 剥离
- 计算器的 cost_from_usage
- PriceCache 命中/未命中

**缺口**: 没有集成测试验证 parser → calculator → DB 的完整链路。`CostBreakdown` 写入 DB 的路径没有 test 覆盖。

#### §9 文档与可维护性

plan.md 是中文设计文档，写得清晰。但当前版本已经与实现存在多处偏差，需要同步。Phase 1/2 的 checkbox 还是 `[ ]`，但代码已实现。模块路径章节 1 描述的是旧路径。

建议: 将 plan.md 中 Section 1、8 更新为实际状态，避免新协作者看到错误的路径信息。

#### §10 结论与下一步行动

**方案整体评分**: 8/10。架构合理，核心实现扎实，主要缺口是 DB 迁移 (router_logs 多维度列) 和文档与实现的偏差。

**必须完成** (blocking):
- [ ] DB 迁移: 为 `router_logs` 添加 per-type token 列和费用列
- [ ] 将 `CostBreakdown` 字段写入 DB (而非只写 total cost)

**强烈建议** (next sprint):
- [ ] 补充 `UnifiedUsage.accepted_prediction_tokens`, `rejected_prediction_tokens`
- [ ] 拆分 `CostBreakdown.cache_cost` 为 `cache_read_cost` + `cache_write_cost`
- [ ] CLI 显示格式 (Section 7) 实现
- [ ] 更新 plan.md Section 1 模块路径和 Section 8 Phase checkboxes

**可以推迟** (backlog):
- [ ] `request_mode` (Batch/Priority 定价) — 需要上游识别批处理请求
- [ ] Gemini audio/image tokens 提取 — 当前用户极少使用

---

### 10.3 CEO 决策记录

| 决策 | 原则 | 结论 |
|------|------|------|
| DB 迁移策略 | 不要 version 后缀 | ALTER TABLE 逐列添加，但新增列应用事务保护 |
| 模块路径偏差 | 代码实际位置优先 | 更新 plan.md Section 1，不移动代码 |
| CostBreakdown 字段合并 | 保持向后兼容 | 拆分为分离字段，但确保 total() 不变 |
| Phase 1/2 checkboxes | 文档反映现实 | 标记为已完成 ✅ |

---

## 11. Eng Review (autoplan — 2026-03-28)

> 自动生成 · 分支: main · 架构 + 测试 + 失败模式

### 11.1 架构现状图

```
burncloud workspace
├── crates/service/crates/service-billing/          ← 计费核心 (Phase 1/2 已实现)
│   ├── src/types.rs                                  UnifiedUsage (10 fields), CostBreakdown (9 fields)
│   ├── src/usage/                                    UsageParser trait + factory
│   │   └── providers/                               5 parsers: openai, anthropic, gemini, deepseek, generic
│   ├── src/calculator.rs                             CostCalculator (price lookup + breakdown)
│   ├── src/cache.rs                                  PriceCache (model, region) → Price
│   └── src/counter.rs                                UsageCounter (streaming accumulation)
│
├── crates/router/src/adaptor/                       ← OpenAI 格式适配层
│   ├── gemini.rs                                     convert_request / convert_response / convert_stream_response
│   └── ...
│
├── crates/database/crates/database-router-log/      ← DB 访问层
│   └── src/lib.rs                                    DbRouterLog (旧结构，不含 per-type 费用列)
│
├── crates/database/src/schema.rs                    ← DDL + 迁移
│   └── router_logs 当前列: prompt_tokens, completion_tokens, cost, model,
│       cache_read_tokens, reasoning_tokens, pricing_region, video_tokens
│
└── crates/server/src/api/log.rs                     ← 对外 API
    ├── GET /console/api/logs
    ├── GET /console/api/usage/{user_id}
    ├── GET /console/internal/billing/summary
    └── POST /console/internal/prices/sync
```

**数据流**:
```
HTTP Request
    → router (crates/router/src/lib.rs)
        → 识别 provider protocol
        → 转发 upstream
        → 接收响应 (stream / non-stream)
        → UsageParser::parse_response / parse_streaming_chunk   [service-billing]
        → CostCalculator::calculate                              [service-billing]
        → RouterLogModel::insert (DbRouterLog)                  [database-router-log]
            ⚠️ CostBreakdown 在此步骤丢失 → 只写 total cost
```

### 11.2 测试覆盖图

| 模块 | Unit Tests | Integration Tests | 覆盖评估 |
|------|-----------|------------------|---------|
| `usage/providers/openai` | 7 tests | 0 | 充分 (streaming, cache, missing) |
| `usage/providers/anthropic` | 6 tests | 0 | 充分 (cache, thinking, extended) |
| `usage/providers/gemini` | 9 tests | 0 | 充分 (SSE prefix, thoughts, cache) |
| `usage/providers/deepseek` | 5 tests | 0 | 基础覆盖 |
| `usage/providers/generic` | 4 tests | 0 | 基础覆盖 |
| `calculator` | 12 tests | 0 | 充分 (per-type pricing) |
| `cache` | 6 tests | 0 | 充分 |
| `router/adaptor/gemini` | 5 tests | 0 | 充分 |
| **parser → calculator → DB 链路** | 0 | **0** | ⚠️ **未覆盖** |
| **streaming accumulation end-to-end** | 0 | **0** | ⚠️ **未覆盖** |

### 11.3 失败模式注册表

| ID | 场景 | 当前行为 | 风险等级 | 建议修复 |
|----|------|---------|---------|---------|
| FM-01 | Gemini streaming — `usageMetadata` 在中间某块出现，不在最后一块 | `UsageCounter` 取最后一个非 None 结果，可能取到中间值 | 中 | Gemini 最终块通常包含完整 usageMetadata，但应取最大值而非最后值 |
| FM-02 | DB 迁移中途失败 (SQLite crash) | 部分列存在，schema 不一致，后续启动可能 panic | 高 | 使用 CREATE + RENAME 事务模式 |
| FM-03 | `prices` 表无此 model 条目 | `calculator` 返回 cost=0，不报错，日志无警告 | 中 | 至少记录 warn!() + 在 API 响应里标记 price_source=Default |
| FM-04 | Gemini 思维链 tokens (`thoughtsTokenCount`) 在计费层 vs. OpenAI 适配层不一致 | adaptor 将 `completion_tokens = candidates + thoughts`；billing parser 将 `output_tokens = candidates only, reasoning_tokens = thoughts`。两层定义不同 | 低 | 文档明确两层语义：adaptor 用于 OpenAI compat，billing 用于成本计算 |
| FM-05 | `force_sync_tx` dead_code 警告 | 编译警告，不影响功能 | 低 | 确认 server init 路径正确注入，或加 `#[allow(dead_code)]` 并注释原因 |
| FM-06 | 流式请求 usage 累加 overflow | `saturating_add` 截断到 i64::MAX，调用方无警告 | 极低 | 在 `saturating_add` 检测 overflow 并 `tracing::warn!` |
| FM-07 | `accepted_prediction_tokens` / `rejected_prediction_tokens` 未提取 | o1 模型的预测 tokens 不参与费用计算，导致费用低估 | 中 (o1 用户) | 在 `UnifiedUsage` 添加两字段，OpenAI parser 提取，calculator 以 output price 计费 |

### 11.4 工程 TODOS

优先级按 P0 > P1 > P2 排序:

#### P0 — 必须完成 (阻塞计费准确性)

```
TODO(eng): DB 迁移 — router_logs per-type 费用列
文件: crates/database/src/schema.rs + crates/database/crates/database-router-log/src/lib.rs
内容: 添加 input_cost, output_cost, cache_cost(拆分为read+write), audio_cost, image_cost,
      video_cost, reasoning_cost, embedding_cost, total_cost 列
      同时添加 cache_write_tokens, audio_input_tokens, audio_output_tokens, image_tokens, embedding_tokens
迁移模式: ALTER TABLE (SQLite) / ALTER TABLE ADD COLUMN IF NOT EXISTS (Postgres)
```

```
TODO(eng): 将 CostBreakdown 持久化到 DB
文件: crates/database/crates/database-router-log/src/lib.rs (RouterLogModel::insert)
内容: 当前只写 cost = breakdown.total()，应写入所有 per-type 费用字段
```

#### P1 — 强烈建议

```
TODO(eng): UnifiedUsage 补充 accepted_prediction_tokens, rejected_prediction_tokens
文件: crates/service/crates/service-billing/src/types.rs
内容: 添加两字段，更新 OpenAI parser, 更新 calculator (以 output_price 计费)
```

```
TODO(eng): CostBreakdown 拆分 cache_cost 为 cache_read_cost + cache_write_cost
文件: crates/service/crates/service-billing/src/types.rs + calculator.rs
内容: 当前 cache_cost = read + write 合并，无法区分。拆分后 billing summary 可分别展示
```

```
TODO(eng): 集成测试 — parser → calculator → DB 链路
文件: crates/service/crates/service-billing/tests/ (新增)
内容: 构造 OpenAI/Anthropic/Gemini 响应 JSON → 验证 DB 写入的 per-type 费用列正确
```

#### P2 — 可以推迟

```
TODO(eng): Gemini audio/image tokens 提取
文件: crates/service/crates/service-billing/src/usage/providers/gemini.rs
内容: 提取 usageMetadata.audioTokensCount, imageTokensCount, videoTokensCount

TODO(eng): request_mode (Batch/Priority) 识别与定价
文件: crates/router/src/lib.rs (从请求 header 识别 batch=true)
内容: 标记 UnifiedUsage.request_mode，calculator 应用 0.5x / 1.7x 折率

TODO(eng): CLI 显示格式 (plan Section 7)
文件: 新 CLI crate 或 crates/server/src/api/
内容: 实现 plan 第 7 节的 Usage Report 展示格式
```

### 11.5 工程决策记录

| 决策点 | 选项 | 选择 | 原因 |
|--------|------|------|------|
| 计费模块位置 | `router/src/usage/` vs `service/service-billing/` | `service-billing` (实际) | 独立 crate 便于测试和复用，更合理 |
| streaming token 累计 | 取最后一块 / 累加 / 取最大 | 取最后非 None (UsageCounter) | Gemini 最终块含完整 metadata，合理 |
| 费用精度 | f64 / Decimal / i64 nanodollar | i64 nanodollar | 无浮点误差，DB 存储简单 |
| DB 迁移安全性 | CREATE+RENAME / ALTER TABLE | ALTER TABLE (当前) | 简单但有风险，P0 改善项 |


---

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/autoplan` (2026-03-28) | 方案范围、优先级、实现偏差 | 1 | issues_open | 5 premises verified: P2 path mismatch, P3 DB migration missing, P4/P5 struct gaps |
| Eng Review | `/autoplan` (2026-03-28) | 架构、测试覆盖、失败模式 | 1 | issues_open | 7 failure modes registered; 3 P0/P1/P2 TODO buckets; 56 unit tests passing |
| Codex Review | — | 独立第二意见 | 0 | — | CODEX_NOT_AVAILABLE in this environment |
| Design Review | — | UI/UX 展示格式 | 0 | — | CLI display (Section 7) deferred to P1 TODO |

**VERDICT: ISSUES_OPEN** — 核心实现扎实 (56 tests, all pass, build clean)。最高优先级缺口:
1. `router_logs` 缺少 per-type 费用列 → `CostBreakdown` 数据被丢弃 (P0)
2. `CostBreakdown` 未写入 DB，只写 `total_cost` (P0)
3. plan.md Section 1 模块路径、Section 8 checkboxes 与实现不符 (低成本修复)

下一步: 执行 Section 11.4 P0 TODOs，然后更新 plan.md Section 1 + 8。

---

<!-- AUTONOMOUS DECISION LOG -->
## Decision Audit Trail

| # | Phase | Decision | Principle | Rationale | Rejected |
|---|-------|----------|-----------|-----------|----------|
| 1 | Phase 0 | Detect UI scope: no UI keywords → skip Design phase | P3 (pragmatic) | Plan has 0 rendering/component terms. CLI display is listed as P1 but has no layout/form/modal terms that need design review | Run design phase |
| 2 | Phase 1 CEO | Mode: SELECTIVE EXPANSION | P1 (completeness) | Plan is a design doc mid-implementation — right mode for scope validation | HOLD_SCOPE, SCOPE_EXPANSION |
| 3 | Phase 1 CEO | Codex unavailable → subagent-only mode | — | CODEX_NOT_AVAILABLE in environment. Tagged `[subagent-only]` in consensus table | — |
| 4 | Phase 1 CEO | Module path mismatch (plan says router/src/usage/, actual is service-billing/) → update doc, not code | P3+P4 (pragmatic, DRY) | Moving code creates blast radius in importers. Update plan.md Section 1 is zero-risk fix | Move code to router/src/usage/ |
| 5 | Phase 1 CEO | Phase 1/2 checkboxes unflagged → flag as P1 doc task | P3 | Low cost, no code risk. Deferred to Section 10.2 §9 | Auto-update checkboxes in plan |
| 6 | Phase 1 CEO | DB migration strategy: flag ALTER TABLE risk → recommend CREATE+RENAME pattern | P1 (completeness) | SQLite DDL not transactional; mid-migration crash leaves partial schema. Mitigating without forcing full migration strategy change | Ignore the risk |
| 7 | Phase 3 Eng | CostBreakdown.cache_cost → split to cache_read_cost + cache_write_cost | P1+P5 (completeness, explicit) | Separate costs are calculable and meaningful for billing reconciliation. Explicit fields beat a merged field | Keep merged cache_cost |
| 8 | Phase 3 Eng | accepted/rejected_prediction_tokens → defer to P1, not P0 | P3 (pragmatic) | o1 users are edge case today; core billing path works without it. P1 because fee underestimate matters | Add immediately to UnifiedUsage |
| 9 | Phase 3 Eng | Integration tests for parser→calculator→DB chain → classify as P1 gap | P1 (completeness) | Unit tests pass but end-to-end chain is untested. CostBreakdown write path unverified | Mark as P2/optional |
| 10 | Phase 3 Eng | FM-04 (Gemini thoughts token semantic difference between adaptor and billing) → document only, no code change | P5 (explicit) | Two layers serve different contracts (OpenAI compat vs billing). Documenting is explicit and sufficient; changing one would break the other | Unify the two definitions |

