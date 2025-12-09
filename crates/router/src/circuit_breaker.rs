use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct UpstreamState {
    failure_count: AtomicU32,
    last_failure_time: Option<Instant>,
}

impl Default for UpstreamState {
    fn default() -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            last_failure_time: None,
        }
    }
}

pub struct CircuitBreaker {
    states: DashMap<String, UpstreamState>,
    failure_threshold: u32,
    cooldown_duration: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, cooldown_seconds: u64) -> Self {
        Self {
            states: DashMap::new(),
            failure_threshold,
            cooldown_duration: Duration::from_secs(cooldown_seconds),
        }
    }

    /// Checks if a request is allowed to proceed to the given upstream.
    pub fn allow_request(&self, upstream_id: &str) -> bool {
        let entry = self.states.entry(upstream_id.to_string()).or_default();

        let current_failures = entry.failure_count.load(Ordering::Relaxed);

        if current_failures < self.failure_threshold {
            return true; // Closed state
        }

        // Circuit is Open, check if cooldown has passed
        if let Some(last_failure) = entry.last_failure_time {
            if last_failure.elapsed() >= self.cooldown_duration {
                // Transition to Half-Open (allow one probe)
                // We don't change state explicitly here, but we allow *this* request.
                // In a real strict implementation, we might want to ensure only ONE request passes,
                // but for simplicity in this stateless router, allowing traffic after cooldown is acceptable.
                // If it fails again, the time updates and we wait again.
                return true;
            }
        }

        false // Still Open
    }

    /// Records a successful request.
    pub fn record_success(&self, upstream_id: &str) {
        if let Some(mut entry) = self.states.get_mut(upstream_id) {
            // Reset failure count on success
            entry.failure_count.store(0, Ordering::Relaxed);
            entry.last_failure_time = None;
        }
    }

    /// Records a failed request.
    pub fn record_failure(&self, upstream_id: &str) {
        let mut entry = self.states.entry(upstream_id.to_string()).or_default();
        let new_count = entry.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        entry.last_failure_time = Some(Instant::now());

        if new_count >= self.failure_threshold {
            println!(
                "Circuit Breaker: Upstream {} tripped! (Failures: {})",
                upstream_id, new_count
            );
        }
    }

    /// Get current health status map for monitoring
    pub fn get_status_map(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        for r in self.states.iter() {
            let count = r.value().failure_count.load(Ordering::Relaxed);
            let status = if count >= self.failure_threshold {
                // Check if in cooldown
                if let Some(last) = r.value().last_failure_time {
                    if last.elapsed() < self.cooldown_duration {
                        format!(
                            "Open (Tripped, {}s left)",
                            (self.cooldown_duration - last.elapsed()).as_secs()
                        )
                    } else {
                        "Half-Open (Probing)".to_string()
                    }
                } else {
                    "Open".to_string()
                }
            } else {
                "Closed (Healthy)".to_string()
            };
            map.insert(r.key().clone(), status);
        }
        map
    }
}
