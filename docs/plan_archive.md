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

---
十四、多货币支持（TODO）

### 问题背景

当前价格系统假设所有价格使用单一货币（美元），但实际场景中：

| 区域 | 货币 | 典型场景 |
|------|------|----------|
| 中国 | 人民币 (CNY) | Qwen 国内版、百度文心、阿里通义 |
| 美国 | 美元 (USD) | OpenAI、Anthropic、Google |
| 欧洲 | 欧元 (EUR) | 部分 European AI 服务 |

**核心问题**：
1. 同一模型在不同区域有不同货币定价
2. 汇率波动导致实际成本计算复杂
3. 用户可能需要按本地货币计费展示

### 业务场景

```
用户 A（中国企业）:
- 调用 Qwen 国内版 → 按 CNY 计费
- 调用 GPT-4 → 按 USD 计费，展示时转换为 CNY

用户 B（美国企业）:
- 调用 Qwen 海外版 → 按 USD 计费
- 调用 GPT-4 → 按 USD 计费
```

### 解决方案

#### 方案 A: 单一基准货币 + 汇率转换

```sql
-- 所有价格以 USD 为基准存储
-- 实时汇率表
CREATE TABLE exchange_rates (
    id INTEGER PRIMARY KEY,
    from_currency TEXT NOT NULL,  -- 'CNY', 'EUR', 'USD'
    to_currency TEXT NOT NULL,    -- 'USD' (基准货币)
    rate REAL NOT NULL,           -- 汇率
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(from_currency, to_currency)
);

-- 价格表增加原货币字段
ALTER TABLE prices ADD COLUMN original_currency TEXT DEFAULT 'USD';
ALTER TABLE prices ADD COLUMN original_input_price REAL;  -- 原币种价格
ALTER TABLE prices ADD COLUMN original_output_price REAL;
```

**优点**：简单，所有内部计算统一用 USD
**缺点**：汇率波动导致展示价格不稳定

#### 方案 B: 多货币独立定价

```sql
-- 价格表增加货币字段
ALTER TABLE prices ADD COLUMN currency TEXT DEFAULT 'USD';

-- 按货币分组的价格
CREATE TABLE prices_by_currency (
    id INTEGER PRIMARY KEY,
    model TEXT NOT NULL,
    currency TEXT NOT NULL,       -- 'USD', 'CNY', 'EUR'
    input_price REAL NOT NULL,
    output_price REAL NOT NULL,
    synced_at DATETIME,
    UNIQUE(model, currency)
);

-- 示例数据
INSERT INTO prices_by_currency VALUES
(1, 'qwen-max', 'CNY', 0.002, 0.006, NOW()),      -- 国内版人民币价格
(2, 'qwen-max', 'USD', 0.0012, 0.006, NOW());     -- 海外版美元价格
```

**优点**：每种货币独立管理，价格稳定
**缺点**：需要维护多套价格数据

#### 方案 C: 渠道级货币配置

```sql
-- 渠道表增加货币配置
ALTER TABLE channels ADD COLUMN pricing_currency TEXT DEFAULT 'USD';

-- 计费时根据渠道货币选择对应价格
-- 展示时按用户偏好货币转换
```

**优点**：与渠道绑定，逻辑清晰
**缺点**：同一模型多渠道时货币可能冲突

### 推荐方案：方案 A + B 混合

```
┌─────────────────────────────────────────────────────────────────────┐
│                      多货币定价架构                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. 价格存储层                                                       │
│     ┌─────────────────────────────────────────────────────────┐    │
│     │ prices 表: 以 USD 为基准价格（从 LiteLLM 同步）            │    │
│     │ prices_by_currency 表: 各区域本地货币价格（手动配置）      │    │
│     └─────────────────────────────────────────────────────────┘    │
│                                                                     │
│  2. 汇率层                                                          │
│     ┌─────────────────────────────────────────────────────────┐    │
│     │ exchange_rates 表: 每日更新汇率                           │    │
│     │ 数据源: 外汇 API 或手动更新                               │    │
│     └─────────────────────────────────────────────────────────┘    │
│                                                                     │
│  3. 计费层                                                          │
│     ┌─────────────────────────────────────────────────────────┐    │
│     │ 内部计算: 统一使用 USD（精度一致）                         │    │
│     │ 日志记录: 同时记录 USD 和本地货币                         │    │
│     └─────────────────────────────────────────────────────────┘    │
│                                                                     │
│  4. 展示层                                                          │
│     ┌─────────────────────────────────────────────────────────┐    │
│     │ 用户偏好: 选择展示货币 (USD/CNY/EUR)                      │    │
│     │ 实时转换: 根据最新汇率计算                               │    │
│     └─────────────────────────────────────────────────────────┘    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 数据结构设计

```rust
/// 货币类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Currency {
    USD,  // 美元（基准货币）
    CNY,  // 人民币
    EUR,  // 欧元
}

/// 多货币价格
pub struct MultiCurrencyPrice {
    pub model: String,
    pub usd_input_price: f64,      // 基准价格（美元）
    pub usd_output_price: f64,
    pub local_currency: Option<Currency>,
    pub local_input_price: Option<f64>,  // 本地货币价格
    pub local_output_price: Option<f64>,
}

/// 汇率服务
pub struct ExchangeRateService {
    rates: DashMap<(Currency, Currency), f64>,  // (from, to) -> rate
    last_updated: DateTime<Utc>,
}

impl ExchangeRateService {
    /// 转换货币
    pub fn convert(&self, amount: f64, from: Currency, to: Currency) -> f64 {
        if from == to {
            return amount;
        }
        let rate = self.rates.get(&(from, to)).map(|r| *r).unwrap_or(1.0);
        amount * rate
    }

    /// 获取汇率（每日更新）
    pub async fn refresh_rates(&mut self) -> Result<()> {
        // 从外部 API 获取汇率
        // 或从数据库加载手动配置的汇率
    }
}
```

### 计费流程变更

```
当前流程:
usage.tokens * price → cost (USD)

新流程:
1. 获取渠道货币配置
2. 查询对应货币价格（优先本地价格，回退 USD）
3. 计算费用（内部用 USD）
4. 记录双货币日志
5. 按用户偏好货币展示
```

### API 响应示例

```json
{
  "usage": {
    "prompt_tokens": 1000,
    "completion_tokens": 500
  },
  "cost": {
    "usd": 0.015,
    "cny": 0.108,
    "currency": "CNY",
    "display": "¥0.11"
  }
}
```

### 实现优先级

| 优先级 | 任务 | 说明 |
|--------|------|------|
| P2 | 数据库增加货币字段 | 支持 USD/CNY |
| P2 | 汇率表设计与实现 | 支持手动更新 |
| P3 | 汇率自动更新 | 接入外汇 API |
| P3 | 用户偏好货币展示 | 前端支持切换 |
| P3 | 多货币报表 | 按货币统计消费 |

### 相关代码位置

- 价格模型: `crates/database-models/src/lib.rs`
- 价格同步: `crates/router/src/price_sync.rs`
- 计费逻辑: `crates/router/src/billing.rs`
- 数据库结构: `crates/database/src/schema.rs`

### 注意事项

1. **精度问题**: 货币计算使用 Decimal 类型或整数（分为单位），避免浮点误差
2. **汇率更新**: 每日更新一次即可，无需实时
3. **审计日志**: 保留原始货币和转换记录，便于审计
4. **合规性**: 某些地区要求展示本地货币价格

---


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

---
十九、feat/price 分支代码规范违规分析与修复

### 问题背景

对 `feat/price` 分支进行代码审查时，发现多处违反 BurnCloud 开发规范的问题。本文档记录所有发现的问题及其修复方案，确保代码质量符合项目标准。

### 问题严重程度分级

| 级别 | 含义 | 处理优先级 |
|------|------|-----------|
| 🔴 P0 | 编译错误/阻塞问题 | 立即修复 |
| 🟠 P1 | 架构违规/重复定义 | 高优先级 |
| 🟡 P2 | 维护性问题 | 中优先级 |
| 🟢 P3 | 技术债务/建议 | 低优先级 |

---

### 🔴 P0: 编译错误

#### 1. lib.rs 重复导出语法错误

**文件**: `crates/router/src/lib.rs:40-41`

**问题**:
```rust
pub use proxy_logic::*;
    proxy_logic, handle_response_with_token_parsing
};
```

**原因**: `pub use proxy_logic::*;` 后面跟着不完整的 `};` 语句，语法错误。

**修复方案**:
```rust
pub use proxy_logic::{proxy_logic, handle_response_with_token_parsing};
```

#### 2. AppState 结构体缺少右尖括号

**文件**: `crates/router/src/lib.rs:82`

**问题**:
```rust
pub config: Arc<RwLock<RouterConfig>,
```

**原因**: 缺少右尖括号 `>`

**修复方案**:
```rust
pub config: Arc<RwLock<RouterConfig>>,
```

#### 3. proxy_logic.rs 同样的语法错误

**文件**: `crates/router/src/proxy_logic.rs:21`

**问题**:
```rust
pub config: Arc<RwLock<RouterConfig>,
```

**修复方案**:
```rust
pub config: Arc<RwLock<RouterConfig>>,
```

---

### 🟠 P1: 重复定义问题

#### 1. AppState 结构体重复定义

**问题**: 同一 crate 内 `AppState` 定义了三次

| 文件 | 行号 |
|------|------|
| `crates/router/src/lib.rs` | 79-92 |
| `crates/router/src/proxy_logic.rs` | 18-31 |
| `crates/router/src/state.rs` | 16-29 |

**违反规范**: 违反"禁止巨型 Crate"和模块组织原则

**修复方案**:
1. 保留 `state.rs` 中的定义作为唯一来源
2. 在 `lib.rs` 中使用 `pub use state::AppState;`
3. 删除 `proxy_logic.rs` 中的重复定义

**修复后**:
```rust
// lib.rs
mod state;
pub use state::AppState;

// proxy_logic.rs
use crate::state::AppState;
```

#### 2. 类型定义重复

**问题**: 多个类型在 `common` 和 `database-models` 两处定义

| 类型 | common 位置 | database-models 位置 |
|------|-------------|---------------------|
| `TieredPrice` | `types.rs:453-468` | `tiered_price.rs:8-18` |
| `Price` | `types.rs:497-542` | `price.rs:9-42` |
| `PriceInput` | `types.rs:546-574` | `price.rs:58-86` |
| `TieredPriceInput` | `types.rs:471-481` | `tiered_price.rs:23-32` |

**违反规范**: 违反四层架构原则，类型应在 Foundation 层 (common) 定义

**修复方案**:
1. 保留 `common/src/types.rs` 中的类型定义
2. `database-models` 中只保留 Model 操作方法（如 `PriceModel::get`, `PriceModel::upsert`）
3. 在 `database-models` 中 `use burncloud_common::types::*` 导入类型

---

### 🟡 P2: Workspace 依赖违规

#### 1. router/Cargo.toml 未使用 workspace 依赖

**文件**: `crates/router/Cargo.toml:31-32`

**问题**:
```toml
futures = "0.3.31"
regex = "1.12.3"
```

**违反规范**: 规范 6.1 要求所有第三方库版本在根 `Cargo.toml` 的 `[workspace.dependencies]` 中声明

**修复方案**:

1. 在根 `Cargo.toml` 添加:
```toml
[workspace.dependencies]
futures = "0.3"
regex = "1"
```

2. 修改 `router/Cargo.toml`:
```toml
futures.workspace = true
regex.workspace = true
```

#### 2. common/Cargo.toml 未使用 workspace 依赖

**文件**: `crates/common/Cargo.toml:20`

**问题**:
```toml
bcrypt = "0.15"
```

**修复方案**:

1. 在根 `Cargo.toml` 添加:
```toml
bcrypt = "0.15"
```

2. 修改 `common/Cargo.toml`:
```toml
bcrypt.workspace = true
```

#### 3. dev-dependencies 未使用 workspace

**文件**: `crates/router/Cargo.toml:35-38`

**问题**:
```toml
[dev-dependencies]
mockito = "1.7.1"
tempfile = "3"
```

**修复方案**:

1. 在根 `Cargo.toml` 添加:
```toml
mockito = "1.7"
tempfile = "3"
```

2. 修改 `router/Cargo.toml`:
```toml
[dev-dependencies]
mockito.workspace = true
tempfile.workspace = true
```

---

### 🟢 P3: 技术债务

#### 1. TODO 遗留

**文件**: `crates/service/crates/service-inference/src/lib.rs:103`

**问题**:
```rust
// TODO: 这里应该等待 health check 成功才标记为 Running
```

**违反规范**: 规范明确禁止遗留 `TODO` 除非用户明确要求占位

**修复方案**: 实现健康检查或移除 TODO 注释

#### 2. 测试代码中大量使用 .unwrap()

**文件**: `crates/cli/src/price.rs` 等

**问题**: 虽然规范允许在测试中使用 `.unwrap()`，但使用过于密集

**建议**: 考虑使用 `expect()` 提供更好的错误上下文

---

### 修复清单

#### P0 编译错误（必须立即修复）

| # | 文件 | 行号 | 问题 | 状态 |
|---|------|------|------|------|
| 1 | `router/src/lib.rs` | 40-41 | 重复导出语法错误 | ⬜ 待修复 |
| 2 | `router/src/lib.rs` | 82 | 缺少 `>` | ⬜ 待修复 |
| 3 | `router/src/proxy_logic.rs` | 21 | 缺少 `>` | ⬜ 待修复 |

#### P1 架构问题

| # | 问题 | 涉及文件 | 状态 |
|---|------|---------|------|
| 1 | AppState 重复定义 | lib.rs, proxy_logic.rs, state.rs | ⬜ 待修复 |
| 2 | Price 类型重复 | types.rs, price.rs | ⬜ 待修复 |
| 3 | TieredPrice 类型重复 | types.rs, tiered_price.rs | ⬜ 待修复 |
| 4 | PriceInput 类型重复 | types.rs, price.rs | ⬜ 待修复 |

#### P2 Workspace 依赖

| # | 依赖 | 添加位置 | 状态 |
|---|------|---------|------|
| 1 | futures | 根 Cargo.toml | ⬜ 待修复 |
| 2 | regex | 根 Cargo.toml | ⬜ 待修复 |
| 3 | bcrypt | 根 Cargo.toml | ⬜ 待修复 |
| 4 | mockito | 根 Cargo.toml | ⬜ 待修复 |
| 5 | tempfile | 根 Cargo.toml | ⬜ 待修复 |

---

### 验证方案

#### 编译验证
```bash
# 修复后运行
cargo build
cargo clippy -- -D warnings
cargo test
```

#### 架构验证
```bash
# 确认无重复定义
grep -r "pub struct AppState" crates/
grep -r "pub struct Price " crates/
grep -r "pub struct TieredPrice" crates/
```

#### Workspace 依赖验证
```bash
# 确认所有依赖使用 workspace
grep -r '= "' crates/*/Cargo.toml
grep -r '= "' crates/*/crates/*/Cargo.toml
```

---

### 相关文件

| 文件 | 改动类型 |
|------|---------|
| `crates/router/src/lib.rs` | 语法修复、删除重复定义 |
| `crates/router/src/proxy_logic.rs` | 语法修复、删除重复定义 |
| `crates/router/src/state.rs` | 保留作为 AppState 唯一定义 |
| `crates/database/crates/database-models/src/price.rs` | 删除类型定义，保留 Model |
| `crates/database/crates/database-models/src/tiered_price.rs` | 删除类型定义，保留 Model |
| `crates/database/crates/database-models/src/lib.rs` | 更新导入 |
| `crates/router/Cargo.toml` | 使用 workspace 依赖 |
| `crates/common/Cargo.toml` | 使用 workspace 依赖 |
| `Cargo.toml` (根) | 添加 workspace 依赖 |

---

## 十六、CLI 命令行工具完善

### 背景

当前 CLI 工具已支持部分功能，但还有一些 API 功能缺少对应的 CLI 命令。为方便大模型进行黑盒测试和运维操作，需要补全缺失的 CLI 命令。

### 已有 CLI 命令

| 模块 | 命令 | 说明 |
|------|------|------|
| **channel** | add, list, show, delete | 渠道管理 |
| **price** | list, set, get, show, delete, sync-status, import, export, validate | 价格管理 |
| **tiered** | list-tiers, add-tier, import-tiered, delete-tiers, check-tiered | 阶梯定价 |
| **token** | list, create, update, delete | API Token 管理 |
| **protocol** | list, add, delete, show, test | 协议配置 |
| **currency** | list-rates, set-rate, refresh, convert | 汇率管理 |

### 缺失 CLI 命令（按优先级）

#### P0 - 高优先级

##### 1. user (用户管理)

```bash
# 用户注册
burncloud user register --username <name> --password <pwd> --email <email>

# 用户登录
burncloud user login --username <name> --password <pwd>

# 列出所有用户
burncloud user list [--limit 100] [--offset 0]

# 用户充值
burncloud user topup --user-id <id> --amount <amount> --currency <USD|CNY>

# 充值记录
burncloud user recharges --user-id <id> [--limit 100]

# 检查用户名
burncloud user check-username --username <name>
```

**实现文件**:
- `crates/cli/src/user.rs` (新建)
- `crates/cli/src/commands.rs` (添加 user 子命令)

**对应 API**:
- `POST /console/api/user/register`
- `POST /console/api/user/login`
- `GET /console/api/list_users`
- `POST /console/api/user/topup`
- `GET /console/api/user/recharges`

---

#### P1 - 中优先级

##### 2. channel update (补全渠道管理)

```bash
# 更新渠道配置
burncloud channel update <id> \
  [--name <name>] \
  [--key <key>] \
  [--status <1|2|3>] \
  [--models <models>] \
  [--priority <n>] \
  [--pricing-region <cn|intl|universal>]
```

**实现文件**:
- `crates/cli/src/channel.rs` (添加 cmd_channel_update)
- `crates/cli/src/commands.rs` (添加 update 子命令)

---

##### 3. group (路由组管理)

```bash
# 创建路由组
burncloud group create --name <name> [--members <member1,member2>]

# 列出所有组
burncloud group list [--format table|json]

# 显示组详情
burncloud group show <id>

# 删除组
burncloud group delete <id> [-y]

# 查看组成员
burncloud group members <id>

# 设置组成员
burncloud group members <id> --set <member1,member2>
```

**实现文件**:
- `crates/cli/src/group.rs` (新建)
- `crates/cli/src/commands.rs` (添加 group 子命令)

**对应 API**:
- `POST /groups`
- `GET /groups`
- `GET /groups/{id}`
- `DELETE /groups/{id}`
- `GET/PUT /groups/{id}/members`

---

#### P2 - 低优先级

##### 4. log (日志管理)

```bash
# 列出请求日志
burncloud log list [--user-id <id>] [--limit 100] [--offset 0]

# 用户使用统计
burncloud log usage --user-id <id>
```

**实现文件**:
- `crates/cli/src/log.rs` (新建)
- `crates/cli/src/commands.rs` (添加 log 子命令)

---

##### 5. monitor (系统监控)

```bash
# 显示系统监控指标
burncloud monitor status [--format table|json]
```

**实现文件**:
- `crates/cli/src/monitor.rs` (新建)
- `crates/cli/src/commands.rs` (添加 monitor 子命令)

---

### 实现顺序

```
Phase 1 (P0):
├── user register
├── user login
├── user list
├── user topup
└── user recharges

Phase 2 (P1):
├── channel update
├── group create
├── group list
├── group show
├── group delete
└── group members

Phase 3 (P2):
├── log list
├── log usage
└── monitor status
```

### 代码模板

#### user.rs 模板

```rust
//! User management CLI commands

use anyhow::Result;
use burncloud_database::Database;
use clap::ArgMatches;

/// Handle user register command
pub async fn cmd_user_register(db: &Database, args: &ArgMatches) -> Result<()> {
    let username = args.get_one::<String>("username").unwrap();
    let password = args.get_one::<String>("password").unwrap();
    let email = args.get_one::<String>("email");

    // TODO: Implement user registration
    println!("User '{}' registered successfully", username);
    Ok(())
}

/// Handle user list command
pub async fn cmd_user_list(db: &Database, args: &ArgMatches) -> Result<()> {
    let limit: i32 = args.get_one::<String>("limit").unwrap_or(&"100".to_string()).parse()?;
    let offset: i32 = args.get_one::<String>("offset").unwrap_or(&"0".to_string()).parse()?;

    // TODO: Implement user listing
    println!("Listing users (limit={}, offset={})", limit, offset);
    Ok(())
}

/// Handle user command routing
pub async fn handle_user_command(db: &Database, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("register", sub_m)) => cmd_user_register(db, sub_m).await,
        Some(("list", sub_m)) => cmd_user_list(db, sub_m).await,
        // ... other subcommands
        _ => {
            println!("User management commands:");
            println!("  register      Register a new user");
            println!("  login         User login");
            println!("  list          List all users");
            println!("  topup         Top up user balance");
            println!("  recharges     List recharge history");
            Ok(())
        }
    }
}
```

### 相关文件

| 文件 | 改动类型 |
|------|---------|
| `crates/cli/src/user.rs` | 新建 |
| `crates/cli/src/group.rs` | 新建 |
| `crates/cli/src/log.rs` | 新建 |
| `crates/cli/src/monitor.rs` | 新建 |
| `crates/cli/src/channel.rs` | 添加 update 命令 |
| `crates/cli/src/commands.rs` | 添加所有新子命令 |
| `crates/cli/src/lib.rs` | 导出新模块 |

---

## 十九、Price 模块 CLI 功能完善

### 背景

当前 `price` CLI 命令存在以下问题：
1. `price get` 不支持 `--region` 参数，导致查询有区域的定价时返回空
2. `price list` 不支持 `--region` 过滤
3. `price delete` 不支持 `--region` 参数，无法删除特定区域的价格
4. `price set` 缺少 `--priority-input/output` 和 `--audio-input` 参数

### 当前功能状态

| 命令 | 功能 | 状态 |
|------|------|------|
| `price set` | 设置价格 | ✅ 支持 region, cache, batch |
| `price list` | 列出价格 | ⚠️ 不支持 --region 过滤 |
| `price get` | 查询价格 | ❌ 不支持 --region |
| `price show` | 显示详情 | ✅ 支持 --region |
| `price delete` | 删除价格 | ⚠️ 不支持 --region |
| `tiered add-tier` | 阶梯定价 | ✅ 支持 --region |

### 缺失功能清单

#### P0 - 必须修复（Bug）

##### 1. price get --region

**问题**: `price get` 查询时硬编码 `region=None`，无法查询特定区域的价格

```bash
# 当前行为（错误）
$ burncloud price set test-model --input 1.0 --output 2.0 --region cn
✓ Price set for 'test-model': USD input=1.0000/1M, output=2.0000/1M [cn]

$ burncloud price get test-model --currency USD
No USD price found for model 'test-model'  # ❌ 找不到

# 期望行为
$ burncloud price get test-model --currency USD --region cn
Model: test-model
Currency: USD
Input Price: 1.0000/1M tokens
Output Price: 2.0000/1M tokens
Region: cn
```

**实现文件**:
- `crates/cli/src/price.rs` (修改 `price get` 处理逻辑)
- `crates/cli/src/commands.rs` (添加 `--region` 参数)

**改动点**:
- Line 152: `PriceModel::get(db, model, curr, None)` → `PriceModel::get(db, model, curr, region)`
- Line 190: `PriceModel::get_all_currencies(db, model, None)` → 添加 region 参数支持

---

##### 2. price list --region

**问题**: `price list` 无法按区域过滤，管理大量价格时不便

```bash
# 期望行为
$ burncloud price list --region cn
Model                          Currency    Input ($/1M)   Output ($/1M)     Region
--------------------------------------------------------------------------------
deepseek-chat                       CNY          0.1400          0.2800         cn
qwen-max                            CNY          2.5880         10.3390         cn

$ burncloud price list --region international
Model                          Currency    Input ($/1M)   Output ($/1M)     Region
--------------------------------------------------------------------------------
deepseek-chat                       USD          0.0190          0.0380 international
qwen-max                            USD          5.0000         20.0000 international
```

**实现文件**:
- `crates/cli/src/price.rs` (修改 `price list` 处理逻辑)
- `crates/cli/src/commands.rs` (添加 `--region` 参数)
- `crates/database/crates/database-models/src/price.rs` (list 函数添加 region 参数)

**改动点**:
- `PriceModel::list()` 函数签名添加 `region: Option<&str>` 参数
- CLI handler 传递 region 参数

---

#### P1 - 建议修复

##### 3. price delete --region

**问题**: `price delete` 删除模型的所有价格，无法删除特定区域

```bash
# 当前行为
$ burncloud price delete test-model
✓ All prices deleted for 'test-model'  # 删除所有区域

# 期望行为
$ burncloud price delete test-model --region cn
✓ Deleted cn region price for 'test-model'

$ burncloud price delete test-model  # 不带 --region 删除所有
✓ All prices deleted for 'test-model'
```

**实现文件**:
- `crates/cli/src/price.rs` (修改 `price delete` 处理逻辑)
- `crates/cli/src/commands.rs` (添加 `--region` 参数)
- `crates/database/crates/database-models/src/price.rs` (添加 delete_by_region 函数)

---

#### P2 - 可选增强

##### 4. price set 高级定价参数

**问题**: `price set` 缺少 `priority` 和 `audio` 价格参数

```bash
# 期望行为
$ burncloud price set gpt-4o \
    --input 2.5 --output 10.0 \
    --priority-input 4.25 \
    --priority-output 17.0 \
    --audio-input 17.5
```

**新增参数**:
| 参数 | 说明 | 类型 |
|------|------|------|
| `--priority-input` | 优先输入价格 ($/1M tokens) | f64 |
| `--priority-output` | 优先输出价格 ($/1M tokens) | f64 |
| `--audio-input` | 音频输入价格 ($/1M tokens) | f64 |

**实现文件**:
- `crates/cli/src/price.rs` (添加参数解析)
- `crates/cli/src/commands.rs` (添加参数定义)

---

### 实现顺序

```
Phase 1 (P0 Bug 修复):
├── price get --region
└── price list --region

Phase 2 (P1 功能增强):
└── price delete --region

Phase 3 (P2 可选):
├── price set --priority-input
├── price set --priority-output
└── price set --audio-input
```

### 代码改动详情

#### commands.rs 改动

```rust
// price get 子命令添加 --region
Command::new("get")
    .about("Get price for a model")
    .arg(Arg::new("model").required(true))
    .arg(Arg::new("currency").long("currency"))
    .arg(Arg::new("region")        // 新增
        .long("region")
        .help("Filter by region (cn, international)"))
    .arg(Arg::new("verbose").short('v').long("verbose"))

// price list 子命令添加 --region
Command::new("list")
    .about("List all prices")
    .arg(Arg::new("limit").long("limit").default_value("100"))
    .arg(Arg::new("offset").long("offset").default_value("0"))
    .arg(Arg::new("currency").long("currency"))
    .arg(Arg::new("region")        // 新增
        .long("region")
        .help("Filter by region (cn, international)"))

// price delete 子命令添加 --region
Command::new("delete")
    .about("Delete price for a model")
    .arg(Arg::new("model").required(true))
    .arg(Arg::new("region")        // 新增
        .long("region")
        .help("Delete only for a specific region"))

// price set 子命令添加高级参数
Command::new("set")
    // ... 现有参数 ...
    .arg(Arg::new("priority-input")    // 新增
        .long("priority-input")
        .help("Priority input price per 1M tokens"))
    .arg(Arg::new("priority-output")   // 新增
        .long("priority-output")
        .help("Priority output price per 1M tokens"))
    .arg(Arg::new("audio-input")       // 新增
        .long("audio-input")
        .help("Audio input price per 1M tokens"))
```

#### price.rs (CLI) 改动

```rust
// price get 处理
Some(("get", sub_m)) => {
    let model = sub_m.get_one::<String>("model").unwrap();
    let currency = sub_m.get_one::<String>("currency").map(|s| s.as_str());
    let region = sub_m.get_one::<String>("region").map(|s| s.as_str());  // 新增
    // ...

    match PriceModel::get(db, model, curr, region).await? {  // 传递 region
        // ...
    }
}

// price list 处理
Some(("list", sub_m)) => {
    let currency = sub_m.get_one::<String>("currency").map(|s| s.as_str());
    let region = sub_m.get_one::<String>("region").map(|s| s.as_str());  // 新增

    let prices = PriceModel::list(db, limit, offset, currency, region).await?;  // 传递 region
    // ...
}

// price set 处理 - 新增参数
let priority_input_price: Option<f64> = sub_m
    .get_one::<String>("priority-input")
    .and_then(|s| s.parse().ok());
let priority_output_price: Option<f64> = sub_m
    .get_one::<String>("priority-output")
    .and_then(|s| s.parse().ok());
let audio_input_price: Option<f64> = sub_m
    .get_one::<String>("audio-input")
    .and_then(|s| s.parse().ok());
```

#### price.rs (Database) 改动

```rust
// list 函数添加 region 参数
pub async fn list(
    db: &Database,
    limit: i32,
    offset: i32,
    currency: Option<&str>,
    region: Option<&str>,  // 新增
) -> Result<Vec<Price>> {
    // 根据 region 构建 SQL WHERE 条件
}

// 新增 delete_by_region 函数
pub async fn delete_by_region(
    db: &Database,
    model: &str,
    region: &str,
) -> Result<u64> {
    let sql = "DELETE FROM prices WHERE model = ? AND region = ?";
    // ...
}
```

### 测试用例

```bash
# P0 测试
# 1. price get --region
burncloud price set test-model --input 1.0 --output 2.0 --region cn
burncloud price get test-model --currency USD --region cn
# 期望: 显示 cn 区域价格

# 2. price list --region
burncloud price list --region cn
# 期望: 仅显示 cn 区域价格

# P1 测试
# 3. price delete --region
burncloud price delete test-model --region cn
# 期望: 仅删除 cn 区域，保留其他区域

# P2 测试
# 4. price set 高级参数
burncloud price set gpt-4o --input 2.5 --output 10.0 --priority-input 4.25
burncloud price get gpt-4o -v
# 期望: 显示 priority input price
```

### 相关文件

| 文件 | 改动类型 |
|------|---------|
| `crates/cli/src/price.rs` | 修改 get/list/delete/set 处理逻辑 |
| `crates/cli/src/commands.rs` | 添加新参数定义 |
| `crates/database/crates/database-models/src/price.rs` | list 添加 region 参数，新增 delete_by_region |
