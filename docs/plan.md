 一、方案对比分析

 1. 价格获取方案对比

 ┌────────────┬────────────────────────┬────────────────────┐
 │    维度    │        用户方案        │      现有规划      │
 ├────────────┼────────────────────────┼────────────────────┤
 │ 数据来源   │ LiteLLM 社区数据库同步 │ 手动配置 prices 表 │
 ├────────────┼────────────────────────┼────────────────────┤
 │ 新模型处理 │ 自动填充 + 通知管理员  │ 无明确流程         │
 ├────────────┼────────────────────────┼────────────────────┤
 │ 维护成本   │ 低（社区维护）         │ 高（需人工更新）   │
 ├────────────┼────────────────────────┼────────────────────┤
 │ 精确度     │ 依赖社区更新时效       │ 完全可控           │
 └────────────┴────────────────────────┴────────────────────┘

 结论：用户方案的创新点——接入 LiteLLM 价格数据库——强烈建议采纳。这是 plan.md 未考虑的优化方向。

 2. 限流处理方案对比

 ┌──────────┬──────────────────────────────┬─────────────────────────────────┐
 │   维度   │           用户方案           │            现有规划             │
 ├──────────┼──────────────────────────────┼─────────────────────────────────┤
 │ 信息来源 │ 解析上游响应头 X-RateLimit-* │ 无响应头解析                    │
 ├──────────┼──────────────────────────────┼─────────────────────────────────┤
 │ 学习机制 │ 自适应算法动态调整限制       │ 静态配置                        │
 ├──────────┼──────────────────────────────┼─────────────────────────────────┤
 │ 状态管理 │ 单一限流状态                 │ 两级（渠道级+模型级）           │
 ├──────────┼──────────────────────────────┼─────────────────────────────────┤
 │ 错误分类 │ 仅限流相关                   │ 完整分类（401/402/404/429/5xx） │
 └──────────┴──────────────────────────────┴─────────────────────────────────┘

 结论：两个方案互补。
 - 现有规划在错误分类和两级状态管理上更完善
 - 用户方案在限流学习机制上更先进

 3. 整体架构对比

 用户方案重点：
 ┌─────────────────────────────────────┐
 │  外部数据源 → 自动配置 → 智能学习   │
 │  (LiteLLM价格)   (新模型)  (限流)   │
 └─────────────────────────────────────┘

 现有规划重点：
 ┌─────────────────────────────────────┐
 │  内部状态 → 错误分类 → 多活选择     │
 │  (两级状态)  (影响范围)  (负载均衡) │
 │  注意：禁止降级，无可用渠道返回错误  │
 └─────────────────────────────────────┘

 ---
 二、代码现状验证

 根据代码探索结果，当前实现：

 ┌────────────┬──────────────────────┬───────────────────────────────────┐
 │    功能    │         状态         │             关键问题              │
 ├────────────┼──────────────────────┼───────────────────────────────────┤
 │ 熔断器     │ 简单计数             │ 所有失败同等对待，无类型分类      │
 ├────────────┼──────────────────────┼───────────────────────────────────┤
 │ 限流器     │ 用户侧限流           │ 不处理上游 429                    │
 ├────────────┼──────────────────────┼───────────────────────────────────┤
 │ 路由选择   │ 静态 priority/weight │ 无渠道状态过滤                    │
 ├────────────┼──────────────────────┼───────────────────────────────────┤
 │ 响应头解析 │ 无                   │ 不解析 retry-after、x-ratelimit-* │
 ├────────────┼──────────────────────┼───────────────────────────────────┤
 │ 价格管理   │ 有 prices 表         │ 需手动维护                        │
 └────────────┴──────────────────────┴───────────────────────────────────┘

 ---
 三、整合建议

 方案整合架构

 ┌─────────────────────────────────────────────────────────────────────┐
 │                      BurnCloud 智能路由系统 v2                       │
 ├─────────────────────────────────────────────────────────────────────┤
 │                                                                     │
 │  ┌─────────────────────────────────────────────────────────────┐   │
 │  │                 外部数据层 (用户方案贡献)                      │   │
 │  │  • LiteLLM 价格数据库同步 (每小时)                            │   │
 │  │  • 新模型自动发现与通知                                       │   │
 │  └─────────────────────────────────────────────────────────────┘   │
 │                              ↓                                      │
 │  ┌─────────────────────────────────────────────────────────────┐   │
 │  │                 状态管理层 (现有规划核心)                      │   │
 │  │  • ChannelStateTracker (渠道级状态)                          │   │
 │  │  • ModelState (模型级状态)                                    │   │
 │  │  • 错误类型分类 (401/402/404/429/5xx)                        │   │
 │  └─────────────────────────────────────────────────────────────┘   │
 │                              ↓                                      │
 │  ┌─────────────────────────────────────────────────────────────┐   │
 │  │                 学习层 (用户方案贡献)                          │   │
 │  │  • AdaptiveRateLimit (响应头学习)                             │   │
 │  │  • 状态机: LEARNING → STABLE → COOLDOWN                      │   │
 │  └─────────────────────────────────────────────────────────────┘   │
 │                              ↓                                      │
 │  ┌─────────────────────────────────────────────────────────────┐   │
 │  │                 路由层 (现有规划改进)                          │   │
 │  │  • 状态过滤                                                   │   │
 │  │  • 多活选择（同一模型多渠道负载均衡）                          │   │
 │  │  • 加权选择                                                   │   │
 │  │  • 禁止降级：无可用渠道时直接返回错误                          │   │
 │  └─────────────────────────────────────────────────────────────┘   │
 │                                                                     │
 └─────────────────────────────────────────────────────────────────────┘

 数据结构整合

 /// 整合后的模型状态 (合并 plan.md + 用户方案)
 pub struct ModelState {
     // 来自 plan.md
     pub model: String,
     pub channel_id: i32,
     pub status: ModelStatus,           // Available/RateLimited/QuotaExhausted/...
     pub rate_limit_until: Option<Instant>,

     // 来自用户方案 - 自适应限流
     pub adaptive_limit: AdaptiveRateLimit,

     // 统计
     pub success_count: u32,
     pub failure_count: u32,
     pub avg_latency_ms: u32,
 }

 /// 用户方案新增
 pub struct AdaptiveRateLimit {
     pub learned_limit: Option<u32>,    // 从响应头学习
     pub current_limit: u32,            // 当前应用限制
     pub success_streak: u32,
     pub state: RateLimitState,         // Learning/Stable/Cooldown
     pub last_adjusted_at: DateTime<Utc>,
 }

 ---
 四、实施优先级建议

 Phase 1: 核心能力增强 (P0 - 对齐现有规划)

 ┌────────────────┬──────────┬──────────────┐
 │      任务      │   来源   │     原因     │
 ├────────────────┼──────────┼──────────────┤
 │ 两级状态管理   │ plan.md  │ 基础架构     │
 ├────────────────┼──────────┼──────────────┤
 │ 错误类型分类   │ plan.md  │ 影响范围区分 │
 ├────────────────┼──────────┼──────────────┤
 │ 渠道状态过滤   │ plan.md  │ 路由智能化   │
 ├────────────────┼──────────┼──────────────┤
 │ 429 响应头解析 │ 用户方案 │ 学习上游限制 │
 └────────────────┴──────────┴──────────────┘

 Phase 2: 自动化能力 (P1 - 用户方案创新)

 ┌──────────────────┬──────────┬──────────────┐
 │       任务       │   来源   │     原因     │
 ├──────────────────┼──────────┼──────────────┤
 │ LiteLLM 价格同步 │ 用户方案 │ 降低维护成本 │
 ├──────────────────┼──────────┼──────────────┤
 │ 自适应限流算法   │ 用户方案 │ 自动优化     │
 ├──────────────────┼──────────┼──────────────┤
 │ 新模型自动配置   │ 用户方案 │ 运维自动化   │
 └──────────────────┴──────────┴──────────────┘

 Phase 3: 高级特性 (P2)

 ┌──────────────┬──────────┐
 │     任务     │   来源   │
 ├──────────────┼──────────┤
 │ 成本预测预警 │ 用户方案 │
 ├──────────────┼──────────┤
 │ 语义缓存     │ plan.md  │
 ├──────────────┼──────────┤
 │ A/B 测试     │ plan.md  │
 └──────────────┴──────────┘

 ---
 五、关键文件变更清单

 ┌──────────────────────────────────────┬──────────┬──────────────────────┐
 │                 文件                 │ 变更类型 │         说明         │
 ├──────────────────────────────────────┼──────────┼──────────────────────┤
 │ crates/router/src/circuit_breaker.rs │ 重构     │ 支持失败类型分类     │
 ├──────────────────────────────────────┼──────────┼──────────────────────┤
 │ crates/router/src/channel_state.rs   │ 新增     │ ChannelStateTracker  │
 ├──────────────────────────────────────┼──────────┼──────────────────────┤
 │ crates/router/src/adaptive_limit.rs  │ 新增     │ 自适应限流算法       │
 ├──────────────────────────────────────┼──────────┼──────────────────────┤
 │ crates/router/src/model_router.rs    │ 改进     │ 集成状态过滤         │
 ├──────────────────────────────────────┼──────────┼──────────────────────┤
 │ crates/router/src/lib.rs             │ 改进     │ 解析响应头、记录状态 │
 ├──────────────────────────────────────┼──────────┼──────────────────────┤
 │ crates/router/src/price_sync.rs      │ 新增     │ LiteLLM 价格同步     │
 ├──────────────────────────────────────┼──────────┼──────────────────────┤
 │ crates/database/src/schema.rs        │ 扩展     │ 价格表增加同步字段   │
 ├──────────────────────────────────────┼──────────┼──────────────────────┤
 │ crates/database-models/src/lib.rs    │ 扩展     │ 价格同步逻辑         │
 └──────────────────────────────────────┴──────────┴──────────────────────┘

 ---
 六、验证方式

 1. 单元测试
   - 错误类型分类正确性
   - 自适应限流状态机转换
   - 价格同步数据解析
 2. 集成测试
   - 429 响应头解析 → 状态更新 → 路由选择
   - 新模型首次请求 → 自动配置流程
 3. 端到端测试
   - 模拟上游 429 → 验证限流学习
   - 添加新模型 → 验证价格自动填充

 ---
 七、结论

 用户方案与现有规划高度互补：

 1. 现有规划 (docs/plan.md) 提供了完善的两级状态管理和错误分类架构
 2. 用户方案 补充了价格自动获取和自适应限流学习机制

 建议将用户方案的以下内容整合进 docs/plan.md：
 - LiteLLM 价格数据库同步
 - 响应头解析 (X-RateLimit-*, Retry-After)
 - 自适应限流状态机
 - 新模型自动配置流程

 ---
 八、429 限流方案优化

 现有方案回顾

 计划文件中已包含：
 - AdaptiveRateLimit 数据结构
 - RateLimitInfo 响应头解析
 - 状态机 LEARNING → STABLE → COOLDOWN

 优化点分析

 1. 区分账户级和模型级限流

 ┌─────────────────────────────────────────────────────────────────────┐
 │                    429 限流的两级影响范围                             │
 ├─────────────────────────────────────────────────────────────────────┤
 │                                                                     │
 │  账户级限流（影响整个渠道）                                          │
 │  ├─ 触发条件：响应体包含 "rate limit exceeded for account"          │
 │  ├─ 响应头：X-RateLimit-Limit（账户总配额）                         │
 │  ├─ 影响：该渠道所有模型暂停使用                                    │
 │  └─ 存储：ChannelState.account_rate_limit_until                    │
 │                                                                     │
 │  模型级限流（只影响特定模型）                                        │
 │  ├─ 触发条件：响应体包含 "rate limit exceeded for model 'xxx'"      │
 │  ├─ 响应头：X-RateLimit-Limit（模型配额）                           │
 │  ├─ 影响：只暂停该渠道的该模型                                      │
 │  └─ 存储：ModelState.rate_limit_until                              │
 │                                                                     │
 └─────────────────────────────────────────────────────────────────────┘

 2. 自适应算法参数细化

 /// 自适应限流配置参数
 pub struct AdaptiveLimitConfig {
     /// 学习阶段时长（默认 4 小时）
     pub learning_duration: Duration,

     /// 学习阶段初始限制（保守值，默认 10 RPM）
     pub initial_limit: u32,

     /// 单次调整步长（默认 10%）
     pub adjustment_step: f32,

     /// 连续成功次数阈值（默认 20 次）
     pub success_threshold: u32,

     /// 触发冷却的连续失败次数（默认 3 次）
     pub failure_threshold: u32,

     /// 冷却时长（默认 60 秒）
     pub cooldown_duration: Duration,

     /// 冷却后恢复比例（默认 50%）
     pub recovery_ratio: f32,

     /// 最大限制（安全上限，默认 1000 RPM）
     pub max_limit: u32,
 }

 3. 状态机完整转换规则

 ┌─────────────────────────────────────────────────────────────────────┐
 │                    自适应限流状态机                                   │
 ├─────────────────────────────────────────────────────────────────────┤
 │                                                                     │
 │  ┌─────────┐                                                       │
 │  │ LEARNING│ ← 新渠道/新模型初始状态                               │
 │  └────┬────┘                                                       │
 │       │                                                            │
 │       │ 触发条件：                                                 │
 │       │ • 学习时长 ≥ learning_duration                             │
 │       │ • 且 success_count > 0                                    │
 │       │                                                            │
 │       ▼                                                            │
 │  ┌─────────┐                                                       │
 │  │ STABLE  │ ← 正常工作状态                                        │
 │  └────┬────┘                                                       │
 │       │                                                            │
 │       │ 触发条件：                                                 │
 │       │ • 连续 429 次数 ≥ failure_threshold                        │
 │       │                                                            │
 │       ▼                                                            │
 │  ┌─────────┐                                                       │
 │  │ COOLDOWN│ ← 冷却期，暂停使用                                    │
 │  └────┬────┘                                                       │
 │       │                                                            │
 │       │ 触发条件：                                                 │
 │       │ • 冷却时长 ≥ cooldown_duration                             │
 │       │                                                            │
 │       ▼                                                            │
 │  ┌─────────┐                                                       │
 │  │ LEARNING│ ← 从 50% 限制重新开始学习                             │
 │  └─────────┘                                                       │
 │                                                                     │
 └─────────────────────────────────────────────────────────────────────┘

 4. 算法核心逻辑

 impl AdaptiveRateLimit {
     /// 处理成功响应
     pub fn on_success(&mut self, headers: &HeaderMap, config: &AdaptiveLimitConfig) {
         // 1. 从响应头学习上游限制
         if let Some(limit) = parse_header_limit(headers) {
             self.learned_limit = Some(limit);
         }

         // 2. 更新连续成功计数
         self.success_streak += 1;
         self.failure_streak = 0;

         // 3. 状态转换：LEARNING → STABLE
         if self.state == RateLimitState::Learning {
             let elapsed = self.last_adjusted_at.elapsed();
             if elapsed >= config.learning_duration && self.success_count > 0 {
                 self.state = RateLimitState::Stable;
                 log::info!("Rate limit state: LEARNING → STABLE, limit={}", self.current_limit);
             }
         }

         // 4. 尝试提升限制（仅 LEARNING 状态）
         if self.state == RateLimitState::Learning {
             if self.success_streak >= config.success_threshold {
                 let new_limit = (self.current_limit as f32 * (1.0 + config.adjustment_step)) as u32;
                 let max = self.learned_limit.unwrap_or(config.max_limit);
                 self.current_limit = new_limit.min(max);
                 self.success_streak = 0;
                 self.last_adjusted_at = Instant::now();
             }
         }
     }

     /// 处理 429 限流响应
     pub fn on_rate_limited(
         &mut self,
         retry_after: Option<Duration>,
         scope: RateLimitScope,
         config: &AdaptiveLimitConfig,
     ) {
         // 1. 更新失败计数
         self.failure_streak += 1;
         self.success_streak = 0;

         // 2. 降低当前限制
         let new_limit = (self.current_limit as f32 * 0.8) as u32;
         self.current_limit = new_limit.max(config.initial_limit);

         // 3. 检查是否进入冷却
         if self.failure_streak >= config.failure_threshold {
             self.state = RateLimitState::Cooldown;
             self.cooldown_until = Instant::now() + config.cooldown_duration;
             log::warn!("Rate limit state: {} → COOLDOWN, limit={}",
                 if self.failure_streak == config.failure_threshold { "LEARNING/STABLE" } else { "COOLDOWN" },
                 self.current_limit);
         }

         // 4. 记录限流解除时间
         self.rate_limit_until = retry_after.map(|d| Instant::now() + d);
     }

     /// 检查是否可用
     pub fn check_available(&self) -> bool {
         // 检查冷却状态
         if self.state == RateLimitState::Cooldown {
             if let Some(until) = self.cooldown_until {
                 if Instant::now() < until {
                     return false;
                 }
                 // 冷却结束，准备恢复
             }
         }

         // 检查限流时间
         if let Some(until) = self.rate_limit_until {
             if Instant::now() < until {
                 return false;
             }
         }

         true
     }

     /// 冷却结束后恢复
     pub fn recover_from_cooldown(&mut self, config: &AdaptiveLimitConfig) {
         self.state = RateLimitState::Learning;
         self.current_limit = (self.current_limit as f32 * config.recovery_ratio) as u32;
         self.current_limit = self.current_limit.max(config.initial_limit);
         self.failure_streak = 0;
         self.cooldown_until = None;
         self.last_adjusted_at = Instant::now();
         log::info!("Rate limit state: COOLDOWN → LEARNING, limit={}", self.current_limit);
     }
 }

 5. 上游响应头解析（各供应商差异）

 ┌─────────────────────────────────────────────────────────────────────┐
 │                    各供应商 429 响应格式                              │
 ├─────────────────────────────────────────────────────────────────────┤
 │                                                                     │
 │  OpenAI                                                             │
 │  ┌──────────────────────────────────────────────────────────────┐  │
 │  │ HTTP/1.1 429 Too Many Requests                               │  │
 │  │ X-RateLimit-Limit-Requests: 500                              │  │
 │  │ X-RateLimit-Limit-Tokens: 150000                             │  │
 │  │ X-RateLimit-Remaining-Requests: 0                            │  │
 │  │ X-RateLimit-Remaining-Tokens: 0                              │  │
 │  │ Retry-After: 20                                              │  │
 │  │                                                              │  │
 │  │ {"error": {"message": "Rate limit exceeded...",              │  │
 │  │           "type": "rate_limit_exceeded"}}                    │  │
 │  └──────────────────────────────────────────────────────────────┘  │
 │                                                                     │
 │  Anthropic                                                          │
 │  ┌──────────────────────────────────────────────────────────────┐  │
 │  │ HTTP/1.1 429 Too Many Requests                               │  │
 │  │ anthropic-ratelimit-requests-limit: 1000                     │  │
 │  │ anthropic-ratelimit-requests-remaining: 0                    │  │
 │  │ anthropic-ratelimit-requests-reset: 2024-01-01T00:00:00Z     │  │
 │  │ retry-after: 30                                              │  │
 │  │                                                              │  │
 │  │ {"error": {"type": "rate_limit_error",                       │  │
 │  │           "message": "Rate limit exceeded..."}}              │  │
 │  └──────────────────────────────────────────────────────────────┘  │
 │                                                                     │
 │  Azure OpenAI                                                       │
 │  ┌──────────────────────────────────────────────────────────────┐  │
 │  │ HTTP/1.1 429 Too Many Requests                               │  │
 │  │ X-RateLimit-Limit: 500                                       │  │
 │  │ X-RateLimit-Remaining: 0                                     │  │
 │  │ X-RateLimit-Reset: 1708948800                                │  │
 │  │                                                              │  │
 │  │ {"error": {"code": "429", "message": "Rate limit..."}}       │  │
 │  └──────────────────────────────────────────────────────────────┘  │
 │                                                                     │
 │  Google Gemini                                                      │
 │  ┌──────────────────────────────────────────────────────────────┐  │
 │  │ HTTP/1.1 429 Too Many Requests                               │  │
 │  │ (无标准响应头，需解析响应体)                                   │  │
 │  │                                                              │  │
 │  │ {"error": {"code": 429,                                      │  │
 │  │           "message": "Quota exceeded...",                    │  │
 │  │           "status": "RESOURCE_EXHAUSTED"}}                   │  │
 │  └──────────────────────────────────────────────────────────────┘  │
 │                                                                     │
 └─────────────────────────────────────────────────────────────────────┘

 6. 响应头解析器实现

 /// 统一的限流信息解析
 pub fn parse_rate_limit_info(
     headers: &HeaderMap,
     body: &str,
     channel_type: ChannelType,
 ) -> RateLimitInfo {
     match channel_type {
         ChannelType::OpenAI => parse_openai_rate_limit(headers, body),
         ChannelType::Azure => parse_azure_rate_limit(headers, body),
         ChannelType::Anthropic | ChannelType::Aws => parse_anthropic_rate_limit(headers, body),
         ChannelType::Gemini | ChannelType::VertexAI => parse_gemini_rate_limit(body),
         _ => parse_generic_rate_limit(headers),
     }
 }

 fn parse_openai_rate_limit(headers: &HeaderMap, body: &str) -> RateLimitInfo {
     RateLimitInfo {
         // Requests per minute
         request_limit: headers.get("x-ratelimit-limit-requests")
             .and_then(|v| v.to_str().ok())
             .and_then(|v| v.parse().ok()),
         // Tokens per minute
         token_limit: headers.get("x-ratelimit-limit-tokens")
             .and_then(|v| v.to_str().ok())
             .and_then(|v| v.parse().ok()),
         // Retry after seconds
         retry_after: headers.get("retry-after")
             .and_then(|v| v.to_str().ok())
             .and_then(|v| v.parse::<u64>().ok())
             .map(Duration::from_secs),
         // Parse error scope from body
         scope: parse_rate_limit_scope(body),
     }
 }

 fn parse_anthropic_rate_limit(headers: &HeaderMap, body: &str) -> RateLimitInfo {
     RateLimitInfo {
         request_limit: headers.get("anthropic-ratelimit-requests-limit")
             .and_then(|v| v.to_str().ok())
             .and_then(|v| v.parse().ok()),
         token_limit: None, // Anthropic uses different model
         retry_after: headers.get("retry-after")
             .and_then(|v| v.to_str().ok())
             .and_then(|v| v.parse::<u64>().ok())
             .map(Duration::from_secs),
         scope: parse_rate_limit_scope(body),
     }
 }

 /// 从错误响应体判断限流范围
 fn parse_rate_limit_scope(body: &str) -> RateLimitScope {
     if body.contains("account") || body.contains("API key") {
         RateLimitScope::Account
     } else if body.contains("model") {
         RateLimitScope::Model
     } else {
         RateLimitScope::Unknown
     }
 }

 7. 多渠道限流协调

 当同一模型有多个渠道时，需要协调限流状态：

 impl ChannelStateTracker {
     /// 获取可用的渠道列表（考虑限流状态）
     pub fn get_available_channels(&self, model: &str, candidates: &[Channel]) -> Vec<Channel> {
         candidates.iter()
             .filter(|ch| {
                 let state = self.get_channel_state(ch.id);

                 // 检查渠道级限流
                 if !state.auth_ok { return false; }
                 if let Some(until) = state.account_rate_limit_until {
                     if Instant::now() < until { return false; }
                 }

                 // 检查模型级限流
                 if let Some(model_state) = state.models.get(model) {
                     if !model_state.adaptive_limit.check_available() {
                         return false;
                     }
                 }

                 true
             })
             .cloned()
             .collect()
     }

     /// 按限流状态排序（优先选择"更健康"的渠道）
     pub fn sort_by_health(&self, model: &str, channels: &mut [Channel]) {
         channels.sort_by(|a, b| {
             let state_a = self.get_model_health_score(a.id, model);
             let state_b = self.get_model_health_score(b.id, model);
             state_b.partial_cmp(&state_a).unwrap_or(std::cmp::Ordering::Equal)
         });
     }

     /// 计算渠道健康分数（0.0-1.0，越高越健康）
     fn get_model_health_score(&self, channel_id: i32, model: &str) -> f32 {
         let state = self.get_channel_state(channel_id);

         // 基础分数
         let mut score = 1.0;

         // 考虑限流状态
         if let Some(model_state) = state.models.get(model) {
             let adaptive = &model_state.adaptive_limit;

             // 当前限制/学习限制的比例
             if let Some(learned) = adaptive.learned_limit {
                 score *= adaptive.current_limit as f32 / learned as f32;
             }

             // 连续成功加分
             score += (adaptive.success_streak as f32 / 100.0).min(0.2);

             // 连续失败减分
             score -= (adaptive.failure_streak as f32 / 10.0).min(0.5);
         }

         score.clamp(0.0, 1.0)
     }
 }

 实现任务补充

 ┌────────┬───────────────────────────────────┬───────────────────────────────┐
 │ 优先级 │               任务                │             文件              │
 ├────────┼───────────────────────────────────┼───────────────────────────────┤
 │ P0     │ 实现 AdaptiveLimitConfig 配置结构 │ router/src/adaptive_limit.rs  │
 ├────────┼───────────────────────────────────┼───────────────────────────────┤
 │ P0     │ 实现完整的状态机转换逻辑          │ router/src/adaptive_limit.rs  │
 ├────────┼───────────────────────────────────┼───────────────────────────────┤
 │ P0     │ 实现各供应商响应头解析器          │ router/src/response_parser.rs │
 ├────────┼───────────────────────────────────┼───────────────────────────────┤
 │ P1     │ 实现渠道健康分数计算              │ router/src/channel_state.rs   │
 ├────────┼───────────────────────────────────┼───────────────────────────────┤
 │ P1     │ 实现按健康分数排序选择            │ router/src/model_router.rs    │
 └────────┴───────────────────────────────────┴───────────────────────────────┘

 ---
 九、关键业务约束

 禁止降级：客户请求的模型必须精确交付，不允许降级到其他模型或替代模型。

 这意味着：
 - 渠道状态追踪的目的是选择最优可用渠道，而非降级选择
 - 当所有渠道不可用时，直接返回错误而非降级
 - 多渠道配置用于负载均衡和高可用，而非降级兜底

 ---
 九、新模型全自动适应方案

 业务约束（用户确认）

 1. 模型匹配策略：只支持精确匹配，不使用通配符匹配
 2. 新模型价格：新模型默认不可用，除非 LiteLLM 数据库中已有价格

 核心挑战：API 接口变化自动适应

 问题场景：Azure 更新上游接口，从 /v1/chat/completions 变成 /v1/responses，如何不修改代码自动适应？

 解决方案：可配置协议适配器

 架构设计

 ┌─────────────────────────────────────────────────────────────────────┐
 │                 可配置协议适配器架构                                  │
 ├─────────────────────────────────────────────────────────────────────┤
 │                                                                     │
 │  传统方式（硬编码）：                                                │
 │  ┌──────────────────────────────────────────────────────────────┐  │
 │  │  ChannelType → 代码中的固定路径 → 固定请求格式                │  │
 │  │  Azure       → /v1/chat/completions → OpenAI 格式            │  │
 │  │  （需要修改代码才能适应新接口）                                │  │
 │  └──────────────────────────────────────────────────────────────┘  │
 │                                                                     │
 │  可配置方式（运行时）：                                              │
 │  ┌──────────────────────────────────────────────────────────────┐  │
 │  │  ChannelType → protocol_config 表 → 动态路径/格式映射        │  │
 │  │  Azure       → { endpoint: "/v1/responses", ... }            │  │
 │  │  （通过配置或自动检测适应新接口，无需修改代码）                │  │
 │  └──────────────────────────────────────────────────────────────┘  │
 │                                                                     │
 └─────────────────────────────────────────────────────────────────────┘

 数据结构设计

 1. 协议配置表

 CREATE TABLE protocol_configs (
     id INTEGER PRIMARY KEY,
     channel_type INTEGER NOT NULL,         -- 渠道类型
     api_version VARCHAR(32) NOT NULL,      -- API 版本 (如 "2024-02-01", "2025-01-01")
     is_default BOOLEAN DEFAULT FALSE,      -- 是否默认版本

     -- 端点配置
     chat_endpoint VARCHAR(255),            -- 聊天端点 (如 "/v1/chat/completions", "/v1/responses")
     embed_endpoint VARCHAR(255),           -- 嵌入端点
     models_endpoint VARCHAR(255),          -- 模型列表端点

     -- 请求映射 (JSON)
     request_mapping TEXT,                  -- 请求字段映射规则
     -- 响应映射 (JSON)
     response_mapping TEXT,                 -- 响应字段映射规则

     -- 检测规则
     detection_rules TEXT,                  -- 如何检测此版本是否适用

     created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
     updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

     UNIQUE(channel_type, api_version)
 );

 示例配置：

 -- Azure OpenAI 旧版本
 INSERT INTO protocol_configs VALUES (
     1, 3, '2024-02-01', TRUE,
     '/deployments/{deployment_id}/chat/completions',  -- chat_endpoint
     '/deployments/{deployment_id}/embeddings',         -- embed_endpoint
     '/deployments?api-version=2024-02-01',            -- models_endpoint
     '{}',  -- request_mapping (默认 OpenAI 格式)
     '{}',  -- response_mapping (默认 OpenAI 格式)
     '{"endpoint_exists": true}',  -- 检测规则：端点存在
     CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
 );

 -- Azure OpenAI 新版本 (假设)
 INSERT INTO protocol_configs VALUES (
     2, 3, '2025-01-01', FALSE,
     '/deployments/{deployment_id}/responses',          -- 新端点
     '/deployments/{deployment_id}/embeddings',
     '/deployments?api-version=2025-01-01',
     '{"messages": "input", "model": "deployment_id"}', -- 请求映射
     '{"output": "choices[0].message.content"}',        -- 响应映射
     '{"error_code": "ApiVersionDeprecated"}',          -- 检测规则
     CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
 );

 2. 请求/响应映射规则

 /// 请求字段映射（将标准 OpenAI 格式转换为目标格式）
 #[derive(Debug, Deserialize)]
 pub struct RequestMapping {
     /// 字段映射: "目标字段" => "源字段"
     /// 例: {"input": "messages", "deployment_id": "model"}
     pub field_map: HashMap<String, String>,

     /// 需要重命名的字段
     /// 例: {"messages": "input"}
     pub rename: HashMap<String, String>,

     /// 需要添加的固定字段
     /// 例: {"api-version": "2025-01-01"}
     pub add_fields: HashMap<String, Value>,
 }

 /// 响应字段映射（将上游响应转换为标准 OpenAI 格式）
 #[derive(Debug, Deserialize)]
 pub struct ResponseMapping {
     /// 输出字段路径
     /// 例: "choices[0].message.content" 或 "output.text"
     pub content_path: String,

     /// Token 使用字段路径
     pub usage_path: Option<String>,

     /// 错误字段路径
     pub error_path: Option<String>,
 }

 核心实现

 1. 动态适配器工厂 (adaptor/factory.rs)

 pub struct DynamicAdaptorFactory {
     db: Database,
     cache: DashMap<(ChannelType, String), ProtocolConfig>,
 }

 impl DynamicAdaptorFactory {
     pub async fn get_adaptor(&self, channel: &Channel, model: &str) -> Box<dyn Adaptor> {
         // 1. 获取渠道的协议配置
         let config = self.load_protocol_config(channel).await;

         // 2. 创建动态适配器
         Box::new(DynamicAdaptor {
             config,
             channel_type: channel.type_,
         })
     }

     async fn load_protocol_config(&self, channel: &Channel) -> ProtocolConfig {
         // 优先从缓存获取
         if let Some(config) = self.cache.get(&(channel.type_, channel.api_version.clone())) {
             return config.clone();
         }

         // 从数据库加载
         let config = ProtocolConfigModel::get_by_type_version(
             &self.db,
             channel.type_,
             &channel.api_version,
         ).await.unwrap_or_default();

         self.cache.insert((channel.type_, channel.api_version.clone()), config.clone());
         config
     }
 }

 2. 动态适配器实现 (adaptor/dynamic.rs)

 pub struct DynamicAdaptor {
     config: ProtocolConfig,
     channel_type: ChannelType,
 }

 impl Adaptor for DynamicAdaptor {
     async fn prepare_request(&self, req: &mut Request, model: &str) -> Result<()> {
         // 1. 构建动态路径
         let endpoint = self.config.chat_endpoint
             .replace("{deployment_id}", model);

         *req.uri_mut() = endpoint.parse()?;

         // 2. 应用请求映射
         if !self.config.request_mapping.field_map.is_empty() {
             let body = req.body_mut();
             let mut json: Value = serde_json::from_slice(body)?;

             // 应用字段映射规则
             json = apply_mapping(&json, &self.config.request_mapping);

             *body = serde_json::to_vec(&json)?;
         }

         Ok(())
     }

     async fn parse_response(&self, resp: &Response) -> Result<ChatCompletion> {
         let body = resp.body();
         let json: Value = serde_json::from_slice(body)?;

         // 应用响应映射
         let content = extract_value(&json, &self.config.response_mapping.content_path)?;
         let usage = self.config.response_mapping.usage_path
             .as_ref()
             .map(|path| extract_usage(&json, path))
             .transpose()?;

         Ok(ChatCompletion {
             content,
             usage,
             // ...
         })
     }
 }

 3. 自动检测机制 (adaptor/detector.rs)

 pub struct ApiVersionDetector {
     db: Database,
 }

 impl ApiVersionDetector {
     /// 首次请求失败时自动检测正确的 API 版本
     pub async fn detect_and_update(&self, channel: &Channel, error: &ErrorResponse) -> Result<()> {
         // 1. 根据错误信息判断是否需要切换版本
         if let Some(new_version) = self.parse_deprecation_error(error) {
             // 2. 更新渠道的 api_version
             ChannelModel::update_api_version(&self.db, channel.id, &new_version).await?;

             // 3. 清除缓存，下次请求使用新版本配置
             info!("Auto-detected new API version {} for channel {}", new_version, channel.id);
         }

         Ok(())
     }

     fn parse_deprecation_error(&self, error: &ErrorResponse) -> Option<String> {
         // Azure: "Api version 2024-02-01 is deprecated, please use 2025-01-01"
         // OpenAI: 类似的错误格式
         let re = regex!(r"please use (\d{4}-\d{2}-\d{2})");
         re.captures(&error.message).map(|c| c[1].to_string())
     }
 }

 配置管理

 CLI 命令：

 # 查看协议配置
 burncloud protocol list

 # 添加新协议配置
 burncloud protocol add \
     --channel-type azure \
     --api-version 2025-01-01 \
     --chat-endpoint "/deployments/{deployment_id}/responses" \
     --request-mapping '{"messages": "input"}' \
     --response-mapping '{"content_path": "output.text"}'

 # 从 LiteLLM 同步协议配置（如果社区维护）
 burncloud protocol sync

 # 测试协议配置
 burncloud protocol test --channel-id 1 --model "gpt-4"

 Web UI 配置界面：

 ┌─────────────────────────────────────────────────────────────┐
 │  Protocol Configuration                                      │
 ├─────────────────────────────────────────────────────────────┤
 │  Channel Type: [Azure ▼]                                    │
 │  API Version:  [2025-01-01        ]                         │
 │                                                             │
 │  Chat Endpoint: [/deployments/{deployment_id}/responses]   │
 │                                                             │
 │  Request Mapping (JSON):                                    │
 │  ┌─────────────────────────────────────────────────────┐   │
 │  │ {                                                   │   │
 │  │   "field_map": {"input": "messages"},               │   │
 │  │   "add_fields": {"api-version": "2025-01-01"}       │   │
 │  │ }                                                   │   │
 │  └─────────────────────────────────────────────────────┘   │
 │                                                             │
 │  Response Mapping (JSON):                                   │
 │  ┌─────────────────────────────────────────────────────┐   │
 │  │ { "content_path": "output.text" }                   │   │
 │  └─────────────────────────────────────────────────────┘   │
 │                                                             │
 │  [Test Configuration]  [Save]  [Cancel]                     │
 └─────────────────────────────────────────────────────────────┘

 实现任务

 ┌────────┬──────────────────────────┬─────────────────────┐
 │ 优先级 │           任务           │        文件         │
 ├────────┼──────────────────────────┼─────────────────────┤
 │ P0     │ 创建 protocol_configs 表 │ schema.rs           │
 ├────────┼──────────────────────────┼─────────────────────┤
 │ P0     │ 实现 DynamicAdaptor      │ adaptor/dynamic.rs  │
 ├────────┼──────────────────────────┼─────────────────────┤
 │ P0     │ 实现请求/响应映射引擎    │ adaptor/mapping.rs  │
 ├────────┼──────────────────────────┼─────────────────────┤
 │ P1     │ 实现自动检测机制         │ adaptor/detector.rs │
 ├────────┼──────────────────────────┼─────────────────────┤
 │ P1     │ CLI 协议管理命令         │ cli/src/protocol.rs │
 ├────────┼──────────────────────────┼─────────────────────┤
 │ P2     │ Web UI 协议配置界面      │ client-*            │
 └────────┴──────────────────────────┴─────────────────────┘

 简化方案

 ┌─────────────────────────────────────────────────────────────────────┐
 │                     新模型适应流程（简化版）                          │
 ├─────────────────────────────────────────────────────────────────────┤
 │                                                                     │
 │  请求: POST /v1/chat/completions                                    │
 │  Body: { "model": "gpt-4.5-turbo-preview", ... }                   │
 │                     │                                               │
 │                     ▼                                               │
 │  ┌─────────────────────────────────────────────────────────────┐   │
 │  │ Step 1: 精确匹配 abilities 表                                │   │
 │  │                                                             │   │
 │  │ SELECT * FROM abilities                                     │   │
 │  │ WHERE model = 'gpt-4.5-turbo-preview' AND enabled = 1      │   │
 │  │                                                             │   │
 │  │ 结果：                                                       │   │
 │  │ • 找到 → 继续处理                                            │   │
 │  │ • 未找到 → 返回 404 Model Not Found                         │   │
 │  └─────────────────────────────────────────────────────────────┘   │
 │                     │                                               │
 │                     ▼                                               │
 │  ┌─────────────────────────────────────────────────────────────┐   │
 │  │ Step 2: 价格验证                                             │   │
 │  │                                                             │   │
 │  │ 1. 查询 prices 表（精确匹配 model）                          │   │
 │  │ 2. 价格存在 → 继续处理                                        │   │
 │  │ 3. 价格不存在 → 返回 503 Service Unavailable                 │   │
 │  │    + 通知管理员配置价格                                       │   │
 │  └─────────────────────────────────────────────────────────────┘   │
 │                     │                                               │
 │                     ▼                                               │
 │  ┌─────────────────────────────────────────────────────────────┐   │
 │  │ Step 3: 协议适配                                             │   │
 │  │                                                             │   │
 │  │ 根据 ChannelType + api_version 自动选择适配器：              │   │
 │  │ • 从 protocol_configs 表加载配置                             │   │
 │  │ • 应用动态端点和映射规则                                      │   │
 │  │ • 模型名直接透传，不做转换                                    │   │
 │  └─────────────────────────────────────────────────────────────┘   │
 │                     │                                               │
 │                     ▼                                               │
 │  ┌─────────────────────────────────────────────────────────────┐   │
 │  │ Step 4: 首次请求学习                                         │   │
 │  │                                                             │   │
 │  │ 成功响应：学习限流上限（从响应头）                             │   │
 │  │ 失败响应：                                                   │   │
 │  │   • 4xx 错误 → 检测是否需要切换 API 版本                      │   │
 │  │   • 5xx 错误 → 更新渠道状态                                  │   │
 │  └─────────────────────────────────────────────────────────────┘   │
 │                                                                     │
 └─────────────────────────────────────────────────────────────────────┘

 自动化的关键点

 1. Abilities 自动创建（CLI/API）

 当添加新渠道时，自动为渠道配置的模型创建 abilities 记录：

 # CLI 添加渠道时自动创建 abilities
 burncloud channel add -t openai -k "sk-xxx" -m "gpt-4.5-turbo-preview"

 # 自动执行：
 # INSERT INTO abilities (group, model, channel_id, enabled)
 # VALUES ('default', 'gpt-4.5-turbo-preview', 1, 1)

 2. 价格自动同步（每小时）

 从 LiteLLM 同步价格数据，新模型发布后社区通常几天内更新：

 // 每小时执行
 pub async fn sync_prices(&self) -> Result<usize> {
     let data = fetch_litellm_prices().await?;
     for (model, pricing) in data {
         upsert_price(model, pricing.input_price, pricing.output_price);
     }
 }

 3. 协议自动适配

 渠道类型 + API 版本决定协议，支持运行时配置：

 ┌─────────────┬─────────────┬────────────────────────────────────┬───────────────┐
 │ ChannelType │ API Version │                端点                │     映射      │
 ├─────────────┼─────────────┼────────────────────────────────────┼───────────────┤
 │ Azure       │ 2024-02-01  │ /deployments/{id}/chat/completions │ 默认 OpenAI   │
 ├─────────────┼─────────────┼────────────────────────────────────┼───────────────┤
 │ Azure       │ 2025-01-01  │ /deployments/{id}/responses        │ 自定义映射    │
 ├─────────────┼─────────────┼────────────────────────────────────┼───────────────┤
 │ Anthropic   │ 2023-06-01  │ /v1/messages                       │ OpenAI→Claude │
 └─────────────┴─────────────┴────────────────────────────────────┴───────────────┘

 新模型上线流程

 新模型发布（如 gpt-4.5-turbo-preview）
          │
          ▼
 ┌─────────────────────────────────────────┐
 │ 自动流程（无需代码改动）                  │
 ├─────────────────────────────────────────┤
 │ 1. LiteLLM 社区更新价格（通常 1-3 天）   │
 │ 2. BurnCloud 下次同步时获取价格          │
 │ 3. 管理员添加渠道配置该模型              │
 │    burncloud channel add -m "gpt-4.5-*" │
 │ 4. 系统自动创建 abilities 记录           │
 │ 5. 模型可用                              │
 └─────────────────────────────────────────┘

 API 接口变化自动适应：
 ┌─────────────────────────────────────────┐
 │ 1. 请求失败返回弃用错误                   │
 │ 2. 自动检测新 API 版本                   │
 │ 3. 更新渠道的 api_version                │
 │ 4. 下次请求使用新协议                    │
 │                                         │
 │ 或管理员手动配置：                        │
 │ burncloud protocol add --api-version... │
 └─────────────────────────────────────────┘

 数据结构

 model_capabilities 表（可选，用于缓存 LiteLLM 能力数据）

 CREATE TABLE model_capabilities (
     model VARCHAR(255) PRIMARY KEY,
     context_window INTEGER,
     max_output_tokens INTEGER,
     supports_vision BOOLEAN DEFAULT FALSE,
     supports_function_calling BOOLEAN DEFAULT FALSE,
     input_price REAL,
     output_price REAL,
     synced_at DATETIME DEFAULT CURRENT_TIMESTAMP
 );

 实现任务汇总

 ┌────────┬────────────────────────────────────┬────────────────────────────┐
 │ 优先级 │                任务                │            文件            │
 ├────────┼────────────────────────────────────┼────────────────────────────┤
 │ P0     │ CLI channel add 自动创建 abilities │ cli/src/channel.rs         │
 ├────────┼────────────────────────────────────┼────────────────────────────┤
 │ P0     │ 价格同步服务（每小时）             │ router/src/price_sync.rs   │
 ├────────┼────────────────────────────────────┼────────────────────────────┤
 │ P0     │ 创建 protocol_configs 表           │ schema.rs                  │
 ├────────┼────────────────────────────────────┼────────────────────────────┤
 │ P0     │ 实现 DynamicAdaptor                │ adaptor/dynamic.rs         │
 ├────────┼────────────────────────────────────┼────────────────────────────┤
 │ P1     │ 价格不存在时返回 503               │ router/src/lib.rs          │
 ├────────┼────────────────────────────────────┼────────────────────────────┤
 │ P1     │ 新模型通知机制                     │ router/src/notification.rs │
 ├────────┼────────────────────────────────────┼────────────────────────────┤
 │ P1     │ API 版本自动检测                   │ adaptor/detector.rs        │
 ├────────┼────────────────────────────────────┼────────────────────────────┤
 │ P2     │ model_capabilities 表同步          │ router/src/price_sync.rs   │
 ├────────┼────────────────────────────────────┼────────────────────────────┤
 │ P2     │ CLI 协议管理命令                   │ cli/src/protocol.rs        │
 └────────┴────────────────────────────────────┴────────────────────────────┘

 ---
 十、实施计划

 Phase 1: 核心能力增强 (P0)

 目标：建立完善的状态管理和错误处理基础架构

 Step 1.1: 创建渠道状态追踪器

 文件: crates/router/src/channel_state.rs (新建)

 // 核心结构
 pub struct ChannelStateTracker {
     channel_states: DashMap<i32, ChannelState>,
 }

 pub struct ChannelState {
     pub channel_id: i32,
     pub auth_ok: bool,
     pub balance_status: BalanceStatus,
     pub account_rate_limit_until: Option<Instant>,
     pub models: DashMap<String, ModelState>,
 }

 pub struct ModelState {
     pub model: String,
     pub status: ModelStatus,
     pub rate_limit_until: Option<Instant>,
     pub adaptive_limit: AdaptiveRateLimit,  // 整合用户方案
 }

 Step 1.2: 重构熔断器支持错误分类

 文件: crates/router/src/circuit_breaker.rs

 pub enum FailureType {
     AuthFailed,       // 401 - 渠道级
     PaymentRequired,  // 402 - 渠道级
     RateLimited {
         scope: RateLimitScope,  // Account / Model
         retry_after: Option<Duration>,
     },
     ModelNotFound,    // 404 - 模型级
     ServerError,      // 5xx - 临时
     Timeout,          // 网络问题 - 临时
 }

 pub struct UpstreamState {
     pub failure_count: AtomicU32,
     pub failure_type: Option<FailureType>,  // 新增
     pub last_failure_time: Option<Instant>,
 }

 Step 1.3: 实现 429 响应头解析

 文件: crates/router/src/response_parser.rs (新建)

 pub struct RateLimitInfo {
     pub limit: Option<u32>,
     pub remaining: Option<u32>,
     pub reset: Option<DateTime<Utc>>,
     pub retry_after: Option<Duration>,
 }

 pub fn parse_rate_limit_headers(headers: &HeaderMap) -> RateLimitInfo {
     // 解析 X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset
     // 解析 Retry-After
 }

 pub fn parse_error_response(body: &str, provider: ChannelType) -> ErrorInfo {
     // 解析 OpenAI/Anthropic/Gemini 错误格式
     // 判断是账户级还是模型级限流
 }

 Step 1.4: 改进路由选择逻辑

 文件: crates/router/src/model_router.rs

 - 集成 ChannelStateTracker
 - 过滤不可用渠道
 - 多活选择：同一模型多渠道负载均衡（按 priority + weight）
 - 禁止降级：所有渠道不可用时返回错误，不切换到其他模型

 Phase 2: 自动化能力 (P1)

 Step 2.1: LiteLLM 价格同步

 文件: crates/router/src/price_sync.rs (新建)

 pub struct PriceSyncService {
     db: Database,
     http_client: reqwest::Client,
     sync_interval: Duration,  // 1小时
 }

 impl PriceSyncService {
     pub async fn sync_from_litellm(&self) -> Result<usize> {
         // 从 GitHub 获取 model_prices_and_context_window.json
         // 解析并更新 prices 表
     }
 }

 数据源: https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json

 Step 2.2: 自适应限流算法

 文件: crates/router/src/adaptive_limit.rs (新建)

 pub struct AdaptiveRateLimit {
     pub learned_limit: Option<u32>,
     pub current_limit: u32,
     pub state: RateLimitState,
     pub success_streak: u32,
 }

 impl AdaptiveRateLimit {
     pub fn on_success(&mut self, headers: &HeaderMap) { ... }
     pub fn on_rate_limited(&mut self, retry_after: Option<Duration>) { ... }
     pub fn check_available(&self) -> bool { ... }
 }

 ---
 十、关键文件变更总览

 ┌──────────────────────────────────────┬──────┬─────────────────────────┐
 │                 文件                 │ 操作 │          说明           │
 ├──────────────────────────────────────┼──────┼─────────────────────────┤
 │ crates/router/src/channel_state.rs   │ 新建 │ 渠道状态追踪器          │
 ├──────────────────────────────────────┼──────┼─────────────────────────┤
 │ crates/router/src/response_parser.rs │ 新建 │ 响应头/错误体解析       │
 ├──────────────────────────────────────┼──────┼─────────────────────────┤
 │ crates/router/src/adaptive_limit.rs  │ 新建 │ 自适应限流              │
 ├──────────────────────────────────────┼──────┼─────────────────────────┤
 │ crates/router/src/price_sync.rs      │ 新建 │ 价格同步服务            │
 ├──────────────────────────────────────┼──────┼─────────────────────────┤
 │ crates/router/src/circuit_breaker.rs │ 重构 │ 支持错误类型分类        │
 ├──────────────────────────────────────┼──────┼─────────────────────────┤
 │ crates/router/src/model_router.rs    │ 改进 │ 集成状态过滤            │
 ├──────────────────────────────────────┼──────┼─────────────────────────┤
 │ crates/router/src/lib.rs             │ 改进 │ 解析响应头、更新状态    │
 ├──────────────────────────────────────┼──────┼─────────────────────────┤
 │ crates/router/src/lib.rs             │ 改进 │ 启动价格同步定时任务    │
 ├──────────────────────────────────────┼──────┼─────────────────────────┤
 │ crates/database/src/schema.rs        │ 扩展 │ prices 表增加同步时间戳 │
 ├──────────────────────────────────────┼──────┼─────────────────────────┤
 │ docs/plan.md                         │ 更新 │ 整合本方案内容          │
 └──────────────────────────────────────┴──────┴─────────────────────────┘

 ---
 十一、实施后动作

 退出计划模式后，需要执行以下操作：

 将本方案整合到 docs/plan.md：
 1. 添加"价格自动同步"章节
 2. 添加"自适应限流"章节
 3. 添加"可配置协议适配器"章节
 4. 更新实施任务清单

 ---
 十二、验证方式

 1. 编译验证: cargo build
 2. 单元测试: cargo test -p burncloud-router
 3. 集成测试:
   - 模拟 429 响应 → 验证状态更新
   - 模拟响应头 → 验证限流学习
   - 价格同步任务 → 验证数据库更新

---
十三、高级定价维度支持（TODO）

当前价格同步仅支持基础定价字段，LiteLLM JSON 中包含更多高级定价维度：

### 当前已支持

| 字段 | 说明 | 示例 |
|------|------|------|
| `input_cost_per_token` | 标准输入价格 | $2.5/1M tokens |
| `output_cost_per_token` | 标准输出价格 | $10/1M tokens |
| `max_input_tokens` | 上下文窗口 | 128000 |
| `max_output_tokens` | 最大输出 | 16384 |
| `supports_vision` | 支持视觉 | true/false |
| `supports_function_calling` | 支持函数调用 | true/false |

### 当前缺失（需后续实现）

| 字段 | 说明 | 影响场景 |
|------|------|----------|
| `cache_read_input_token_cost` | 缓存命中价格 | Prompt Caching 场景，可节省 90% 成本 |
| `cache_creation_input_token_cost` | 缓存创建价格 | 首次缓存创建 |
| `input_cost_per_token_batches` | 批量请求价格 | Batch API，节省 50% |
| `output_cost_per_token_batches` | 批量输出价格 | Batch API |
| `input_cost_per_token_priority` | 优先级请求价格 | 高优先级请求，加价 70% |
| `output_cost_per_token_priority` | 优先级输出价格 | 高优先级请求 |
| `input_cost_per_audio_token` | 音频输入价格 | 多模态请求，音频贵 7x |
| `search_context_cost_per_query` | 搜索上下文价格 | Web Search 功能 |

### 示例：Claude 3.5 Sonnet 完整定价

```json
{
  "input_cost_per_token": 3e-06,                    // $3.00/1M
  "cache_read_input_token_cost": 3e-07,             // $0.30/1M (缓存命中)
  "cache_creation_input_token_cost": 3.75e-06,      // $3.75/1M (缓存创建)
  "output_cost_per_token": 1.5e-05                   // $15.00/1M
}
```

**影响**: 使用 Prompt Caching 时，缓存命中可节省 90% 输入成本，但当前无法正确计算。

### 解决方案（待定）

#### 方案 A: 扩展现有 prices 表

```sql
ALTER TABLE prices ADD COLUMN cache_read_price REAL;
ALTER TABLE prices ADD COLUMN cache_creation_price REAL;
ALTER TABLE prices ADD COLUMN batch_input_price REAL;
ALTER TABLE prices ADD COLUMN batch_output_price REAL;
ALTER TABLE prices ADD COLUMN priority_input_price REAL;
ALTER TABLE prices ADD COLUMN priority_output_price REAL;
ALTER TABLE prices ADD COLUMN audio_input_price REAL;
ALTER TABLE prices ADD COLUMN full_pricing TEXT;  -- JSON blob for future fields
```

优点：简单直接，查询方便
缺点：字段多，表结构复杂

#### 方案 B: 创建独立价格详情表

```sql
CREATE TABLE price_details (
    id INTEGER PRIMARY KEY,
    model TEXT NOT NULL,
    price_type TEXT NOT NULL,  -- 'standard', 'cache_read', 'batch', 'priority', 'audio'
    input_price REAL,
    output_price REAL,
    UNIQUE(model, price_type)
);
```

优点：灵活扩展，支持无限价格类型
缺点：查询需要 JOIN

#### 方案 C: JSON 字段存储完整定价

```sql
ALTER TABLE prices ADD COLUMN full_pricing TEXT;  -- JSON blob
```

优点：最灵活，无需修改表结构
缺点：查询性能稍差，需要应用层解析

### 复杂计费公式

当前简单公式：
```
cost = prompt_tokens * input_price + completion_tokens * output_price
```

需要支持：
```
cost = standard_tokens * standard_price
     + cache_read_tokens * cache_read_price
     + cache_creation_tokens * cache_creation_price
     + audio_tokens * audio_price
     + batch_tokens * batch_price
     + priority_tokens * priority_price
```

### 实现优先级

| 优先级 | 任务 | 原因 |
|--------|------|------|
| P2 | 缓存定价支持 | Prompt Caching 使用广泛，节省显著 |
| P2 | 批量定价支持 | Batch API 使用广泛 |
| P3 | 优先级定价 | 较少使用 |
| P3 | 音频定价 | 多模态场景 |

### 相关代码位置

- 价格同步: `crates/router/src/price_sync.rs`
- 价格模型: `crates/database/crates/database-models/src/lib.rs`
- 成本计算: `crates/router/src/lib.rs` (proxy_handler)
- 数据库结构: `crates/database/src/schema.rs`

---

### 特殊案例：Qwen 阶梯定价与区域差异

**问题背景**: Qwen3-Max 存在两种复杂定价维度：

#### 1. 区域定价差异

| 区域 | 0-32K 输入 | 32K-128K 输入 | 128K-252K 输入 |
|------|-----------|--------------|----------------|
| 国内版 (北京) | $0.359/1M | $0.574/1M | $1.004/1M |
| 海外版 (新加坡) | $1.2/1M | $2.4/1M | $3.0/1M |

**差异**: 国内版价格约为海外版的 30%

#### 2. 阶梯定价 (Tiered Pricing)

输入 token 越长，单价越高：

```
输入 0-32K tokens:    基准价格
输入 32K-128K tokens: 基准价格 × 2
输入 128K-252K tokens: 基准价格 × 3
```

**示例计算**:
```
用户输入 150K tokens (海外版):
- 前 32K tokens:  32K × $1.2/1M  = $0.0384
- 32K-128K:       96K × $2.4/1M  = $0.2304
- 128K-150K:      22K × $3.0/1M  = $0.0660
- 总计: $0.3348 (而非 150K × $1.2/1M = $0.18)

误差: 当前简单公式会少计费 46%!
```

#### 当前 LiteLLM 数据问题

```json
// LiteLLM 中只有单一价格，无法表达阶梯定价
"dashscope/qwen-max": {
  "input_cost_per_token": 1.6e-06,  // $1.6/1M - 但这是哪个阶梯？
  "max_input_tokens": 30720         // 只显示 30K，实际支持更大
}
```

#### 解决方案

##### 方案 1: 阶梯定价表

```sql
CREATE TABLE tiered_pricing (
    id INTEGER PRIMARY KEY,
    model TEXT NOT NULL,
    region TEXT,                    -- 'cn', 'international', NULL(通用)
    tier_start INTEGER NOT NULL,    -- 阶梯起始 tokens
    tier_end INTEGER NOT NULL,      -- 阶梯结束 tokens
    input_price REAL NOT NULL,      -- 该阶梯输入价格
    output_price REAL NOT NULL,     -- 该阶梯输出价格
    UNIQUE(model, region, tier_start)
);

-- 示例数据
INSERT INTO tiered_pricing VALUES
(1, 'qwen3-max', 'cn', 0, 32000, 0.359, 1.434),
(2, 'qwen3-max', 'cn', 32000, 128000, 0.574, 2.294),
(3, 'qwen3-max', 'cn', 128000, 252000, 1.004, 4.014),
(4, 'qwen3-max', 'international', 0, 32000, 1.2, 6.0),
(5, 'qwen3-max', 'international', 32000, 128000, 2.4, 12.0),
(6, 'qwen3-max', 'international', 128000, 252000, 3.0, 15.0);
```

##### 方案 2: 渠道级价格覆盖

```sql
-- 在 channels 表或 abilities 表中存储区域信息
ALTER TABLE channels ADD COLUMN pricing_region TEXT DEFAULT 'international';
```

##### 方案 3: JSON 配置

```sql
ALTER TABLE prices ADD COLUMN tiered_pricing TEXT;  -- JSON blob

-- 示例
{
  "tiers": [
    {"min": 0, "max": 32000, "input": 1.2, "output": 6.0},
    {"min": 32000, "max": 128000, "input": 2.4, "output": 12.0},
    {"min": 128000, "max": 252000, "input": 3.0, "output": 15.0}
  ],
  "regions": {
    "cn": {"multiplier": 0.3},  -- 国内版价格系数
    "international": {"multiplier": 1.0}
  }
}
```

#### 计费逻辑变更

```rust
// 当前简单公式
fn calculate_cost(tokens: u64, price: f64) -> f64 {
    tokens as f64 * price / 1_000_000.0
}

// 需要支持阶梯计费
fn calculate_tiered_cost(tokens: u64, tiers: &[Tier]) -> f64 {
    let mut cost = 0.0;
    let mut remaining = tokens;

    for tier in tiers {
        let tier_tokens = remaining.min(tier.max - tier.min);
        cost += tier_tokens as f64 * tier.price / 1_000_000.0;
        remaining -= tier_tokens;
        if remaining == 0 { break; }
    }

    cost
}
```

#### 影响范围

| 场景 | 影响程度 |
|------|----------|
| Qwen 系列模型 | 高 - 阶梯定价 |
| DeepSeek 长上下文 | 中 - 可能有类似策略 |
| Gemini 长上下文 | 低 - 目前单一价格 |
| Claude 长上下文 | 低 - 目前单一价格 |

#### 实现优先级

| 优先级 | 任务 |
|--------|------|
| P2 | 阶梯定价表设计与实现 |
| P2 | 阶梯计费逻辑实现 |
| P3 | 区域定价支持 |
| P3 | 自动检测模型是否需要阶梯计费 |
