use dashmap::DashMap;
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
