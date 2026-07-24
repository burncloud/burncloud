use dashmap::DashMap;
use rand::Rng;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// 负载均衡策略
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    /// 轮询
    RoundRobin,
    /// 加权随机
    WeightedRandom,
    /// 最少连接
    LeastConnections,
}

/// 负载均衡器接口
#[allow(dead_code)]
pub trait LoadBalancer: Send + Sync {
    /// 选择下一个通道索引
    /// candidates: 候选通道列表
    /// 返回: 选中的通道索引
    fn select(&self, candidates: &[Upstream]) -> usize;

    /// 获取策略名称
    fn strategy_name(&self) -> &str;
}

/// 上游通道配置
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct Upstream {
    pub id: String,
    pub name: String,
    pub weight: usize,
}

/// 轮询负载均衡器
#[allow(dead_code)]
pub struct RoundRobinBalancer {
    counter: Arc<AtomicUsize>,
}

impl RoundRobinBalancer {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl LoadBalancer for RoundRobinBalancer {
    fn select(&self, candidates: &[Upstream]) -> usize {
        if candidates.is_empty() {
            return 0;
        }

        let current = self.counter.fetch_add(1, Ordering::Relaxed);
        current % candidates.len()
    }

    fn strategy_name(&self) -> &str {
        "round_robin"
    }
}

/// 加权随机负载均衡器
/// 使用线性扫描算法实现加权随机选择，适用于小规模候选集
#[allow(dead_code)]
pub struct WeightedRandomBalancer;

impl WeightedRandomBalancer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
}

impl LoadBalancer for WeightedRandomBalancer {
    fn select(&self, candidates: &[Upstream]) -> usize {
        if candidates.is_empty() {
            return 0;
        }

        if candidates.len() == 1 {
            return 0;
        }

        let weights: Vec<usize> = candidates.iter().map(|c| c.weight).collect();
        let total_weight: usize = weights.iter().sum();

        if total_weight == 0 {
            // 所有权重为0，均匀随机选择
            let mut rng = rand::thread_rng();
            return rng.gen_range(0..candidates.len());
        }

        // 使用线性扫描的加权随机算法（适用于小规模候选集）
        let mut rng = rand::thread_rng();
        let mut random = rng.gen_range(0..total_weight);

        for (i, &weight) in weights.iter().enumerate() {
            if random < weight {
                return i;
            }
            random -= weight;
        }

        // 兜底：返回最后一个
        candidates.len() - 1
    }

    fn strategy_name(&self) -> &str {
        "weighted_random"
    }
}

/// 最少连接负载均衡器
#[allow(dead_code)]
pub struct LeastConnectionsBalancer {
    connections: Arc<DashMap<String, AtomicUsize>>,
}

impl LeastConnectionsBalancer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    /// 增加连接计数
    #[allow(dead_code)]
    pub fn increment(&self, channel_id: &str) {
        let counter = self
            .connections
            .entry(channel_id.to_string())
            .or_insert_with(|| AtomicUsize::new(0));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// 减少连接计数
    #[allow(dead_code)]
    pub fn decrement(&self, channel_id: &str) {
        if let Some(counter) = self.connections.get(channel_id) {
            let current = counter.fetch_sub(1, Ordering::Relaxed);
            if current == 0 {
                // 防止下溢
                counter.store(0, Ordering::Relaxed);
            }
        }
    }

    /// 获取当前连接数
    #[allow(dead_code)]
    pub fn get_connections(&self, channel_id: &str) -> usize {
        self.connections
            .get(channel_id)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

impl LoadBalancer for LeastConnectionsBalancer {
    fn select(&self, candidates: &[Upstream]) -> usize {
        if candidates.is_empty() {
            return 0;
        }

        if candidates.len() == 1 {
            return 0;
        }

        // 找到连接数最少的通道
        let mut min_connections = usize::MAX;
        let mut min_indices = Vec::new();

        for (i, candidate) in candidates.iter().enumerate() {
            let connections = self.get_connections(&candidate.id);
            if connections < min_connections {
                min_connections = connections;
                min_indices.clear();
                min_indices.push(i);
            } else if connections == min_connections {
                min_indices.push(i);
            }
        }

        // 如果有多个通道连接数相同，随机选择一个
        if min_indices.len() == 1 {
            min_indices[0]
        } else {
            let mut rng = rand::thread_rng();
            min_indices[rng.gen_range(0..min_indices.len())]
        }
    }

    fn strategy_name(&self) -> &str {
        "least_connections"
    }
}
