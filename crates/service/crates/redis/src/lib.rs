//! # BurnCloud Service Redis
//!
//! Redis 服务层，提供连接池管理和常用操作封装

use redis::AsyncCommands;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RedisError {
    #[error("Redis Connection Error: {0}")]
    Connection(#[from] redis::RedisError),
    #[error("Configuration Error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, RedisError>;

/// Redis 服务
#[derive(Clone)]
pub struct RedisService {
    client: redis::Client,
    // Redis client manages connection pool internally, but we can wrap it if needed.
    // For now, client.get_multiplexed_async_connection() is efficient.
}

impl RedisService {
    /// 创建新的 Redis 服务实例
    /// url: "redis://127.0.0.1:6379/"
    pub async fn new(url: &str) -> Result<Self> {
        let client = redis::Client::open(url)?;
        // Test connection
        let _conn = client.get_multiplexed_async_connection().await?;
        Ok(Self { client })
    }

    /// 从环境变量创建实例，默认 "redis://127.0.0.1:6379/"
    pub async fn from_env() -> Result<Self> {
        let url = std::env::var("BURNCLOUD_REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        Self::new(&url).await
    }

    /// 获取异步连接
    pub async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(RedisError::Connection)
    }

    /// 设置键值对 (Set)
    pub async fn set(&self, key: &str, value: &str, expire_seconds: Option<u64>) -> Result<()> {
        let mut conn = self.get_connection().await?;
        if let Some(ex) = expire_seconds {
            conn.set_ex::<_, _, ()>(key, value, ex).await?;
        } else {
            conn.set::<_, _, ()>(key, value).await?;
        }
        Ok(())
    }

    /// 获取值 (Get)
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.get_connection().await?;
        let value: Option<String> = conn.get(key).await?;
        Ok(value)
    }

    /// 删除键 (Del)
    pub async fn del(&self, key: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        conn.del::<_, ()>(key).await?;
        Ok(())
    }

    /// 自增 (Incr)
    pub async fn incr(&self, key: &str, delta: i64) -> Result<i64> {
        let mut conn = self.get_connection().await?;
        let val: i64 = conn.incr(key, delta).await?;
        Ok(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redis_connection() {
        // Skip test if no redis available
        // We can assume CI has redis or skip
        // For local dev, we might not have redis running.
        // Let's check connectivity first.

        let service = match RedisService::from_env().await {
            Ok(s) => s,
            Err(_) => {
                println!("Skipping redis test: Connection failed (Is Redis running?)");
                return;
            }
        };

        let key = "test_key_123";
        let val = "hello_world";

        assert!(service.set(key, val, Some(10)).await.is_ok());
        let retrieved = service.get(key).await.unwrap();
        assert_eq!(retrieved, Some(val.to_string()));

        assert!(service.del(key).await.is_ok());
        let retrieved_after = service.get(key).await.unwrap();
        assert_eq!(retrieved_after, None);
    }
}
