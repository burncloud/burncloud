use std::sync::atomic::{AtomicU32, Ordering};

/// Thread-safe token counter for streaming responses.
/// Uses atomic operations for safe concurrent access during streaming.
/// Supports advanced token types for cache pricing (Prompt Caching).
#[derive(Debug, Default)]
pub struct StreamingTokenCounter {
    prompt_tokens: AtomicU32,
    completion_tokens: AtomicU32,
    /// Cache read tokens (Prompt Caching - tokens served from cache)
    cache_read_tokens: AtomicU32,
    /// Cache creation tokens (Prompt Caching - tokens written to cache)
    cache_creation_tokens: AtomicU32,
    /// Audio input tokens
    audio_tokens: AtomicU32,
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
            cache_read_tokens: AtomicU32::new(0),
            cache_creation_tokens: AtomicU32::new(0),
            audio_tokens: AtomicU32::new(0),
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

    /// Sets the cache read tokens count (Prompt Caching).
    pub fn set_cache_read_tokens(&self, count: u32) {
        self.cache_read_tokens.store(count, Ordering::Relaxed);
    }

    /// Sets the cache creation tokens count (Prompt Caching).
    pub fn set_cache_creation_tokens(&self, count: u32) {
        self.cache_creation_tokens.store(count, Ordering::Relaxed);
    }

    /// Sets the audio tokens count.
    pub fn set_audio_tokens(&self, count: u32) {
        self.audio_tokens.store(count, Ordering::Relaxed);
    }

    /// Adds tokens to the counter (updates both prompt and completion if non-zero).
    /// This is useful for parsing usage metadata from responses.
    pub fn add_tokens(&self, prompt_tokens: u32, completion_tokens: u32) {
        if prompt_tokens > 0 {
            self.prompt_tokens.store(prompt_tokens, Ordering::Relaxed);
        }
        if completion_tokens > 0 {
            self.completion_tokens.store(completion_tokens, Ordering::Relaxed);
        }
    }

    /// Adds cache tokens to the counter.
    pub fn add_cache_tokens(&self, cache_read: u32, cache_creation: u32) {
        if cache_read > 0 {
            self.cache_read_tokens.store(cache_read, Ordering::Relaxed);
        }
        if cache_creation > 0 {
            self.cache_creation_tokens.store(cache_creation, Ordering::Relaxed);
        }
    }

    /// Returns the current token usage as (prompt_tokens, completion_tokens).
    pub fn get_usage(&self) -> (u32, u32) {
        (
            self.prompt_tokens.load(Ordering::Relaxed),
            self.completion_tokens.load(Ordering::Relaxed),
        )
    }

    /// Returns the full token usage including cache tokens.
    /// Returns (prompt_tokens, completion_tokens, cache_read_tokens, cache_creation_tokens, audio_tokens)
    pub fn get_full_usage(&self) -> (u32, u32, u32, u32, u32) {
        (
            self.prompt_tokens.load(Ordering::Relaxed),
            self.completion_tokens.load(Ordering::Relaxed),
            self.cache_read_tokens.load(Ordering::Relaxed),
            self.cache_creation_tokens.load(Ordering::Relaxed),
            self.audio_tokens.load(Ordering::Relaxed),
        )
    }

    /// Returns the total token count.
    pub fn total_tokens(&self) -> u32 {
        self.prompt_tokens.load(Ordering::Relaxed) + self.completion_tokens.load(Ordering::Relaxed)
    }

    /// Returns cache token usage as (cache_read_tokens, cache_creation_tokens).
    pub fn get_cache_usage(&self) -> (u32, u32) {
        (
            self.cache_read_tokens.load(Ordering::Relaxed),
            self.cache_creation_tokens.load(Ordering::Relaxed),
        )
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

    #[test]
    fn test_cache_tokens() {
        let counter = StreamingTokenCounter::new();

        counter.set_cache_read_tokens(50);
        counter.set_cache_creation_tokens(20);

        assert_eq!(counter.get_cache_usage(), (50, 20));
    }

    #[test]
    fn test_full_usage() {
        let counter = StreamingTokenCounter::new();

        counter.set_prompt_tokens(100);
        counter.set_completion_tokens(50);
        counter.set_cache_read_tokens(30);
        counter.set_cache_creation_tokens(10);
        counter.set_audio_tokens(5);

        let (prompt, completion, cache_read, cache_creation, audio) = counter.get_full_usage();
        assert_eq!(prompt, 100);
        assert_eq!(completion, 50);
        assert_eq!(cache_read, 30);
        assert_eq!(cache_creation, 10);
        assert_eq!(audio, 5);
    }

    #[test]
    fn test_add_cache_tokens() {
        let counter = StreamingTokenCounter::new();

        counter.add_cache_tokens(100, 50);

        assert_eq!(counter.get_cache_usage(), (100, 50));
    }
}
