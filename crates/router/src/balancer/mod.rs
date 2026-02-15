use dashmap::DashMap;
use rand::Rng;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc; // We need a thread-safe map for state

// Simple Round Robin State
// Key: Group ID, Value: Atomic Counter
pub struct RoundRobinBalancer {
    counters: Arc<DashMap<String, AtomicUsize>>,
}

impl RoundRobinBalancer {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(DashMap::new()),
        }
    }

    pub fn next_index(&self, group_id: &str, group_size: usize) -> usize {
        if group_size == 0 {
            return 0;
        }

        let counter = self
            .counters
            .entry(group_id.to_string())
            .or_insert_with(|| AtomicUsize::new(0));

        let current = counter.fetch_add(1, Ordering::Relaxed);
        current % group_size
    }
}

/// Weighted balancer for weighted random selection
/// Selects items based on their relative weights
pub struct WeightedBalancer<T> {
    items: Vec<(T, u32)>,
    total_weight: u32,
}

impl<T: Clone> WeightedBalancer<T> {
    /// Create a new weighted balancer with items and their weights
    pub fn new(items: Vec<(T, u32)>) -> Self {
        let total_weight: u32 = items.iter().map(|(_, w)| *w).sum();
        Self {
            items,
            total_weight,
        }
    }

    /// Select an item using weighted random selection
    /// Returns None if there are no items
    pub fn select(&self) -> Option<T> {
        if self.items.is_empty() || self.total_weight == 0 {
            return None;
        }

        // Generate random number in [0, total_weight)
        let mut rng = rand::thread_rng();
        let r: u32 = rng.gen_range(0..self.total_weight);

        // Find the item that corresponds to this random number
        let mut cumulative = 0u32;
        for (item, weight) in &self.items {
            cumulative += weight;
            if r < cumulative {
                return Some(item.clone());
            }
        }

        // Fallback to last item (shouldn't happen with proper weights)
        self.items.last().map(|(item, _)| item.clone())
    }

    /// Get the total weight
    pub fn total_weight(&self) -> u32 {
        self.total_weight
    }

    /// Get the number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if there are no items
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin_balancer() {
        let balancer = RoundRobinBalancer::new();

        assert_eq!(balancer.next_index("group1", 3), 0);
        assert_eq!(balancer.next_index("group1", 3), 1);
        assert_eq!(balancer.next_index("group1", 3), 2);
        assert_eq!(balancer.next_index("group1", 3), 0); // Wraps around
    }

    #[test]
    fn test_weighted_balancer_basic() {
        let items = vec![("A".to_string(), 1), ("B".to_string(), 1)];
        let balancer = WeightedBalancer::new(items);

        assert_eq!(balancer.total_weight(), 2);
        assert_eq!(balancer.len(), 2);

        // Should be able to select
        let selected = balancer.select();
        assert!(selected.is_some());
    }

    #[test]
    fn test_weighted_balancer_distribution() {
        // Create balancer with 80:20 weight ratio
        let items = vec![("A".to_string(), 80), ("B".to_string(), 20)];
        let balancer = WeightedBalancer::new(items);

        // Run many selections to check distribution
        let mut count_a = 0;
        let mut count_b = 0;
        let trials = 10000;

        for _ in 0..trials {
            match balancer.select().as_deref() {
                Some("A") => count_a += 1,
                Some("B") => count_b += 1,
                _ => {}
            }
        }

        // Check distribution is approximately correct (within 5%)
        let ratio_a = count_a as f64 / trials as f64;
        let ratio_b = count_b as f64 / trials as f64;

        assert!(ratio_a > 0.75 && ratio_a < 0.85, "A ratio: {}", ratio_a);
        assert!(ratio_b > 0.15 && ratio_b < 0.25, "B ratio: {}", ratio_b);
    }

    #[test]
    fn test_weighted_balancer_empty() {
        let balancer: WeightedBalancer<String> = WeightedBalancer::new(vec![]);
        assert!(balancer.is_empty());
        assert!(balancer.select().is_none());
    }

    #[test]
    fn test_weighted_balancer_zero_weight() {
        let items = vec![("A".to_string(), 0), ("B".to_string(), 0)];
        let balancer = WeightedBalancer::new(items);

        // Total weight is 0, so no selection should be made
        assert!(balancer.select().is_none());
    }
}
