use std::time::Instant;
use dashmap::DashMap;

/// 令牌桶算法实现的限流器
/// Token Bucket implementation for Rate Limiting
pub struct RateLimiter {
    /// 存储每个键（如 UserID/IP）的桶状态
    buckets: DashMap<String, Bucket>,
    /// 默认全局限流配置
    default_capacity: f64,
    default_refill_rate: f64, // tokens per second
}

#[derive(Clone, Copy, Debug)]
struct Bucket {
    tokens: f64,
    last_update: Instant,
}

impl RateLimiter {
    /// 创建新的限流器
    /// capacity: 桶容量 (最大突发请求数)
    /// refill_rate: 每秒填充速率 (RPS/RPM)
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            buckets: DashMap::new(),
            default_capacity: capacity,
            default_refill_rate: refill_rate,
        }
    }

    /// 检查并消耗令牌
    /// key: 限流标识 (如 UserID)
    /// cost:本次请求消耗的令牌数 (通常为 1)
    /// 返回 true 表示允许通过，false 表示限流
    pub fn check(&self, key: &str, cost: f64) -> bool {
        let mut bucket_entry = self.buckets.entry(key.to_string()).or_insert(Bucket {
            tokens: self.default_capacity,
            last_update: Instant::now(),
        });

        let bucket = bucket_entry.value_mut();
        let now = Instant::now();
        
        // 计算时间增量并补充令牌
        let duration = now.duration_since(bucket.last_update).as_secs_f64();
        let new_tokens = duration * self.default_refill_rate;
        
        bucket.tokens = (bucket.tokens + new_tokens).min(self.default_capacity);
        bucket.last_update = now;

        if bucket.tokens >= cost {
            bucket.tokens -= cost;
            true
        } else {
            false
        }
    }

    /// 手动设置特定 Key 的限流规则 (TODO: 支持针对不同用户设置不同限流)
    #[allow(dead_code)]
    pub fn set_custom_limit(&self, _key: &str, _capacity: f64, _refill_rate: f64) {
        // 预留接口
    }
}
