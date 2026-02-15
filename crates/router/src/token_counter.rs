use std::sync::atomic::{AtomicU32, Ordering};

/// Thread-safe token counter for streaming responses.
/// Uses atomic operations for safe concurrent access during streaming.
#[derive(Debug, Default)]
pub struct StreamingTokenCounter {
    prompt_tokens: AtomicU32,
    completion_tokens: AtomicU32,
}

impl StreamingTokenCounter {
    /// Creates a new counter initialized to zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new counter with initial prompt tokens.
    pub fn with_prompt_tokens(prompt_tokens: u32) -> Self {
        Self {
            prompt_tokens: AtomicU32::new(prompt_tokens),
            completion_tokens: AtomicU32::new(0),
        }
    }

    /// Sets the prompt tokens count.
    pub fn set_prompt_tokens(&self, count: u32) {
        self.prompt_tokens.store(count, Ordering::Relaxed);
    }

    /// Increments the completion tokens count by n.
    pub fn increment_completion(&self, n: u32) {
        self.completion_tokens.fetch_add(n, Ordering::Relaxed);
    }

    /// Sets the completion tokens count.
    pub fn set_completion_tokens(&self, count: u32) {
        self.completion_tokens.store(count, Ordering::Relaxed);
    }

    /// Returns the current token usage as (prompt_tokens, completion_tokens).
    pub fn get_usage(&self) -> (u32, u32) {
        (
            self.prompt_tokens.load(Ordering::Relaxed),
            self.completion_tokens.load(Ordering::Relaxed),
        )
    }

    /// Returns the total token count.
    pub fn total_tokens(&self) -> u32 {
        self.prompt_tokens.load(Ordering::Relaxed) + self.completion_tokens.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_basic_counter() {
        let counter = StreamingTokenCounter::with_prompt_tokens(100);
        assert_eq!(counter.get_usage(), (100, 0));

        counter.increment_completion(50);
        assert_eq!(counter.get_usage(), (100, 50));

        counter.increment_completion(25);
        assert_eq!(counter.get_usage(), (100, 75));
    }

    #[test]
    fn test_set_tokens() {
        let counter = StreamingTokenCounter::new();

        counter.set_prompt_tokens(200);
        counter.set_completion_tokens(300);

        assert_eq!(counter.get_usage(), (200, 300));
        assert_eq!(counter.total_tokens(), 500);
    }

    #[test]
    fn test_thread_safety() {
        let counter = Arc::new(StreamingTokenCounter::new());
        let mut handles = vec![];

        // Spawn 10 threads, each incrementing completion tokens by 100
        for _ in 0..10 {
            let counter_clone = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                counter_clone.increment_completion(100);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(counter.get_usage(), (0, 1000));
    }
}
