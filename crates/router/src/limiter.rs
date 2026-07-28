use dashmap::DashMap;
use std::time::{Duration, Instant};

/// 令牌桶算法实现的限流器
/// Token Bucket implementation for Rate Limiting
pub struct RateLimiter {
    /// 存储每个键（如 UserID/IP）的桶状态
    buckets: DashMap<String, Bucket>,
    /// 默认全局限流配置
    default_capacity: f64,
    default_refill_rate: f64, // tokens per second
    /// 清理过期桶的时间阈值（秒）
    cleanup_threshold: Duration,
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
            cleanup_threshold: Duration::from_secs(300), // 5分钟无访问则清理
        }
    }

    /// 定期清理过期桶的方法
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let threshold = self.cleanup_threshold;

        self.buckets.retain(|_key, bucket| {
            now.duration_since(bucket.last_update) < threshold
        });
    }

    /// 检查并消耗令牌
    /// key: 限流标识 (如 UserID)
    /// cost:本次请求消耗的令牌数 (通常为 1)
    /// 返回 true 表示允许通过，false 表示限流
    pub fn check(&self, key: &str, cost: f64) -> bool {
        let mut bucket_entry = self.buckets.entry(key.to_string()).or_insert_with(|| Bucket {
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

    /// 手动设置特定 Key 的限流规则
    /// TODO(issue): 支持针对不同用户设置不同限流
    ///   - Requires: storage for per-user rate limits (database or cache)
    ///   - Consider: integrating with database-user for user-specific configs
    ///   - Consider: hierarchical limits (global -> user -> token)
    #[allow(dead_code)]
    pub fn set_custom_limit(&self, _key: &str, _capacity: f64, _refill_rate: f64) {
        // 预留接口
    }

    /// 获取当前桶数量（用于测试）
    #[cfg(test)]
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(10.0, 1.0);
        
        // 前10次应该允许通过
        for i in 0..10 {
            assert!(limiter.check("test", 1.0), "第 {} 次请求应该通过", i);
        }
        
        // 第11次应该被限流
        assert!(!limiter.check("test", 1.0), "第 11 次请求应该被限流");
    }

    #[test]
    fn test_cleanup_expired_buckets() {
        let limiter = RateLimiter::new(10.0, 1.0);
        
        // 创建一些桶
        limiter.check("user1", 1.0);
        limiter.check("user2", 1.0);
        limiter.check("user3", 1.0);
        
        assert_eq!(limiter.bucket_count(), 3);
        
        // 等待超过清理阈值（5分钟），这里用小阈值测试
        thread::sleep(Duration::from_secs(301));
        
        // 执行清理
        limiter.cleanup_expired();
        
        // 所有桶都应该被清理
        assert_eq!(limiter.bucket_count(), 0);
    }

    #[test]
    fn test_cleanup_only_expired_buckets() {
        let limiter = RateLimiter::new(10.0, 1.0);
        
        // 创建第一个桶
        limiter.check("user1", 1.0);
        
        // 等待一段时间
        thread::sleep(Duration::from_secs(1));
        
        // 创建第二个桶
        limiter.check("user2", 1.0);
        
        assert_eq!(limiter.bucket_count(), 2);
        
        // 等待超过清理阈值（5分钟）
        thread::sleep(Duration::from_secs(301));
        
        // 执行清理前再访问 user2，更新其时间戳
        limiter.check("user2", 1.0);
        
        // 执行清理
        limiter.cleanup_expired();
        
        // user1 应该被清理，user2 应该保留
        assert_eq!(limiter.bucket_count(), 1);
    }

    #[test]
    fn test_bucket_race_condition() {
        let limiter = Arc::new(RateLimiter::new(100.0, 100.0));
        let key = "race_key";
        let num_threads = 10;
        let iterations_per_thread = 100;
        
        // 多个线程同时访问同一个桶
        let handles: Vec<_> = (0..num_threads)
            .map(|_| {
                let limiter = Arc::clone(&limiter);
                thread::spawn(move || {
                    for _ in 0..iterations_per_thread {
                        limiter.check(key, 1.0);
                    }
                })
            })
            .collect();
        
        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }
        
        // 验证桶存在且没有重复创建
        assert_eq!(limiter.bucket_count(), 1);
    }

    #[test]
    fn test_multiple_buckets() {
        let limiter = RateLimiter::new(10.0, 1.0);
        
        // 为不同用户创建桶
        limiter.check("user1", 1.0);
        limiter.check("user2", 1.0);
        
        assert_eq!(limiter.bucket_count(), 2);
        
        // 每个用户独立限流
        for i in 0..10 {
            assert!(limiter.check("user1", 1.0), "user1 第 {} 次应该通过", i);
            assert!(limiter.check("user2", 1.0), "user2 第 {} 次应该通过", i);
        }
        
        assert!(!limiter.check("user1", 1.0), "user1 应该被限流");
        assert!(!limiter.check("user2", 1.0), "user2 应该被限流");
    }
}
