use crate::types::UnifiedUsage;
use std::sync::atomic::{AtomicU64, Ordering};

/// Thread-safe token counter for streaming responses.
///
/// Replaces `StreamingTokenCounter` and supports all token types defined in [`UnifiedUsage`].
/// Settlement strategy: the last streaming chunk that carries usage data is authoritative
/// (set, not accumulate) to avoid double-counting in providers that resend cumulative totals.
#[derive(Debug, Default)]
pub struct UnifiedTokenCounter {
    input_tokens: AtomicU64,
    output_tokens: AtomicU64,
    cache_read_tokens: AtomicU64,
    cache_write_tokens: AtomicU64,
    audio_input_tokens: AtomicU64,
    audio_output_tokens: AtomicU64,
    image_tokens: AtomicU64,
    video_tokens: AtomicU64,
    reasoning_tokens: AtomicU64,
    embedding_tokens: AtomicU64,
}

impl UnifiedTokenCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Overwrite all counters with values from a usage snapshot.
    /// Use this for providers that send cumulative totals (OpenAI, Gemini).
    pub fn set_from_usage(&self, usage: &UnifiedUsage) {
        self.input_tokens
            .store(usage.input_tokens.max(0) as u64, Ordering::Relaxed);
        self.output_tokens
            .store(usage.output_tokens.max(0) as u64, Ordering::Relaxed);
        self.cache_read_tokens
            .store(usage.cache_read_tokens.max(0) as u64, Ordering::Relaxed);
        self.cache_write_tokens
            .store(usage.cache_write_tokens.max(0) as u64, Ordering::Relaxed);
        self.audio_input_tokens
            .store(usage.audio_input_tokens.max(0) as u64, Ordering::Relaxed);
        self.audio_output_tokens
            .store(usage.audio_output_tokens.max(0) as u64, Ordering::Relaxed);
        self.image_tokens
            .store(usage.image_tokens.max(0) as u64, Ordering::Relaxed);
        self.video_tokens
            .store(usage.video_tokens.max(0) as u64, Ordering::Relaxed);
        self.reasoning_tokens
            .store(usage.reasoning_tokens.max(0) as u64, Ordering::Relaxed);
        self.embedding_tokens
            .store(usage.embedding_tokens.max(0) as u64, Ordering::Relaxed);
    }

    /// Accumulate usage from a partial chunk into existing counters.
    /// Use this for providers that send incremental deltas (Anthropic message_start / message_delta).
    /// Individual fields are updated only when non-zero to preserve previous values.
    pub fn accumulate(&self, usage: &UnifiedUsage) {
        if usage.input_tokens > 0 {
            self.input_tokens
                .store(usage.input_tokens as u64, Ordering::Relaxed);
        }
        if usage.output_tokens > 0 {
            self.output_tokens
                .store(usage.output_tokens as u64, Ordering::Relaxed);
        }
        if usage.cache_read_tokens > 0 {
            self.cache_read_tokens
                .store(usage.cache_read_tokens as u64, Ordering::Relaxed);
        }
        if usage.cache_write_tokens > 0 {
            self.cache_write_tokens
                .store(usage.cache_write_tokens as u64, Ordering::Relaxed);
        }
        if usage.audio_input_tokens > 0 {
            self.audio_input_tokens
                .store(usage.audio_input_tokens as u64, Ordering::Relaxed);
        }
        if usage.audio_output_tokens > 0 {
            self.audio_output_tokens
                .store(usage.audio_output_tokens as u64, Ordering::Relaxed);
        }
        if usage.image_tokens > 0 {
            self.image_tokens
                .store(usage.image_tokens as u64, Ordering::Relaxed);
        }
        if usage.video_tokens > 0 {
            self.video_tokens
                .store(usage.video_tokens as u64, Ordering::Relaxed);
        }
        if usage.reasoning_tokens > 0 {
            self.reasoning_tokens
                .store(usage.reasoning_tokens as u64, Ordering::Relaxed);
        }
        if usage.embedding_tokens > 0 {
            self.embedding_tokens
                .store(usage.embedding_tokens as u64, Ordering::Relaxed);
        }
    }

    /// Read the current accumulated usage.
    pub fn get_usage(&self) -> UnifiedUsage {
        UnifiedUsage {
            input_tokens: self.input_tokens.load(Ordering::Relaxed) as i64,
            output_tokens: self.output_tokens.load(Ordering::Relaxed) as i64,
            cache_read_tokens: self.cache_read_tokens.load(Ordering::Relaxed) as i64,
            cache_write_tokens: self.cache_write_tokens.load(Ordering::Relaxed) as i64,
            audio_input_tokens: self.audio_input_tokens.load(Ordering::Relaxed) as i64,
            audio_output_tokens: self.audio_output_tokens.load(Ordering::Relaxed) as i64,
            image_tokens: self.image_tokens.load(Ordering::Relaxed) as i64,
            video_tokens: self.video_tokens.load(Ordering::Relaxed) as i64,
            reasoning_tokens: self.reasoning_tokens.load(Ordering::Relaxed) as i64,
            embedding_tokens: self.embedding_tokens.load(Ordering::Relaxed) as i64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_from_usage() {
        let counter = UnifiedTokenCounter::new();
        let usage = UnifiedUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_tokens: 20,
            reasoning_tokens: 30,
            ..Default::default()
        };
        counter.set_from_usage(&usage);
        let result = counter.get_usage();
        assert_eq!(result.input_tokens, 100);
        assert_eq!(result.output_tokens, 50);
        assert_eq!(result.cache_read_tokens, 20);
        assert_eq!(result.reasoning_tokens, 30);
    }

    #[test]
    fn test_accumulate_two_anthropic_events() {
        let counter = UnifiedTokenCounter::new();
        // message_start sets input
        counter.accumulate(&UnifiedUsage {
            input_tokens: 50,
            cache_read_tokens: 10,
            ..Default::default()
        });
        // message_delta sets output
        counter.accumulate(&UnifiedUsage {
            output_tokens: 75,
            ..Default::default()
        });

        let result = counter.get_usage();
        assert_eq!(result.input_tokens, 50);
        assert_eq!(result.output_tokens, 75);
        assert_eq!(result.cache_read_tokens, 10);
    }

    #[test]
    fn test_set_overwrites_previous() {
        let counter = UnifiedTokenCounter::new();
        counter.set_from_usage(&UnifiedUsage {
            input_tokens: 100,
            ..Default::default()
        });
        counter.set_from_usage(&UnifiedUsage {
            input_tokens: 200,
            output_tokens: 50,
            ..Default::default()
        });
        let result = counter.get_usage();
        assert_eq!(result.input_tokens, 200);
        assert_eq!(result.output_tokens, 50);
    }

    #[test]
    fn test_default_is_all_zeros() {
        let counter = UnifiedTokenCounter::new();
        let result = counter.get_usage();
        assert!(result.is_empty());
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let counter = Arc::new(UnifiedTokenCounter::new());
        let mut handles = vec![];
        for i in 0..10 {
            let c = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                c.set_from_usage(&UnifiedUsage {
                    input_tokens: i,
                    ..Default::default()
                });
            }));
        }
        for h in handles {
            h.join().unwrap_or_else(|e| panic!("thread panicked: {e:?}"));
        }
        // After all threads, some value in 0..10 should be set (no crash)
        let _ = counter.get_usage();
    }
}
