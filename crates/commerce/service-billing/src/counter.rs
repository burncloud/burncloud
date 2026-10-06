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

    /// Add usage deltas to the existing counters.
    ///
    /// This is real saturating addition, so ten events of 100 output tokens
    /// leave 1000 while an overflow caps at `u64::MAX` instead of wrapping back
    /// to a small value. Use it for a provider that sends **incremental deltas**
    /// — i.e. each event reports only what that event produced.
    ///
    /// Negative values are ignored field-by-field, because the counters are
    /// `u64` and a negative delta has no representation here. A zero is a
    /// no-op, which makes this safe for the "event omits this field" case.
    ///
    /// Providers that resend **cumulative totals** (Anthropic's
    /// `message_start` / `message_delta`, OpenAI, Gemini) must use
    /// [`Self::record_cumulative`] or [`Self::set_from_usage`] instead: adding
    /// a running total would bill a multiple of the real usage (#667).
    pub fn accumulate(&self, usage: &UnifiedUsage) {
        self.add_field(&self.input_tokens, usage.input_tokens);
        self.add_field(&self.output_tokens, usage.output_tokens);
        self.add_field(&self.cache_read_tokens, usage.cache_read_tokens);
        self.add_field(&self.cache_write_tokens, usage.cache_write_tokens);
        self.add_field(&self.audio_input_tokens, usage.audio_input_tokens);
        self.add_field(&self.audio_output_tokens, usage.audio_output_tokens);
        self.add_field(&self.image_tokens, usage.image_tokens);
        self.add_field(&self.video_tokens, usage.video_tokens);
        self.add_field(&self.reasoning_tokens, usage.reasoning_tokens);
        self.add_field(&self.embedding_tokens, usage.embedding_tokens);
    }

    /// Saturating `+=` for one field, skipping values that cannot be added.
    fn add_field(&self, counter: &AtomicU64, delta: i64) {
        if delta <= 0 {
            return;
        }

        let delta = delta as u64;
        let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            Some(current.saturating_add(delta))
        });
    }

    /// Record a **cumulative total**: the last non-zero value reported for each
    /// field wins.
    ///
    /// This does **not** add. It is a snapshot update restricted to the fields
    /// the event actually carries, which is what a provider streaming running
    /// totals needs — the final event carries the total, so the counter must
    /// end at it rather than at the sum of every event.
    ///
    /// The `> 0` guard is what makes the pattern work: an event that omits a
    /// field must leave that field alone (Anthropic reports the input side in
    /// `message_start` and the output side in `message_delta`). The cost is
    /// that a field can never be reset back to zero by a later event; use
    /// [`Self::set_from_usage`] for a full replacement.
    ///
    /// If the provider really sends **deltas**, use [`Self::accumulate`].
    pub fn record_cumulative(&self, usage: &UnifiedUsage) {
        self.record_field(&self.input_tokens, usage.input_tokens);
        self.record_field(&self.output_tokens, usage.output_tokens);
        self.record_field(&self.cache_read_tokens, usage.cache_read_tokens);
        self.record_field(&self.cache_write_tokens, usage.cache_write_tokens);
        self.record_field(&self.audio_input_tokens, usage.audio_input_tokens);
        self.record_field(&self.audio_output_tokens, usage.audio_output_tokens);
        self.record_field(&self.image_tokens, usage.image_tokens);
        self.record_field(&self.video_tokens, usage.video_tokens);
        self.record_field(&self.reasoning_tokens, usage.reasoning_tokens);
        self.record_field(&self.embedding_tokens, usage.embedding_tokens);
    }

    /// Store one field when the event reports a positive running total.
    fn record_field(&self, counter: &AtomicU64, total: i64) {
        if total > 0 {
            counter.store(total as u64, Ordering::Relaxed);
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "unit-test module: unwrap/expect failures are the intended failure signal"
)]
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
    fn test_accumulate_adds_deltas() {
        let counter = UnifiedTokenCounter::new();
        counter.accumulate(&UnifiedUsage {
            output_tokens: 100,
            ..Default::default()
        });
        counter.accumulate(&UnifiedUsage {
            output_tokens: 100,
            ..Default::default()
        });

        let result = counter.get_usage();
        assert_eq!(
            result.output_tokens, 200,
            "accumulate is addition: two 100-token deltas are 200"
        );
    }

    #[test]
    fn test_accumulate_ignores_zero_and_negative_deltas() {
        let counter = UnifiedTokenCounter::new();
        counter.accumulate(&UnifiedUsage {
            input_tokens: 100,
            output_tokens: 50,
            ..Default::default()
        });
        counter.accumulate(&UnifiedUsage {
            input_tokens: -5,
            output_tokens: 0,
            ..Default::default()
        });

        let result = counter.get_usage();
        assert_eq!(result.input_tokens, 100, "a negative delta is not a sum");
        assert_eq!(result.output_tokens, 50, "a zero delta is a no-op");
    }

    #[test]
    fn test_add_field_saturates_instead_of_wrapping() {
        let counter = AtomicU64::new(u64::MAX - 1);
        UnifiedTokenCounter::new().add_field(&counter, 10);
        assert_eq!(
            counter.load(Ordering::Relaxed),
            u64::MAX,
            "overflow must saturate at u64::MAX rather than wrapping"
        );
    }

    #[test]
    fn test_record_cumulative_two_anthropic_events() {
        let counter = UnifiedTokenCounter::new();
        // message_start carries the input side as a running total.
        counter.record_cumulative(&UnifiedUsage {
            input_tokens: 50,
            cache_read_tokens: 10,
            ..Default::default()
        });
        // message_delta carries the output side as a running total.
        counter.record_cumulative(&UnifiedUsage {
            output_tokens: 75,
            ..Default::default()
        });

        let result = counter.get_usage();
        assert_eq!(result.input_tokens, 50);
        assert_eq!(result.output_tokens, 75);
        assert_eq!(result.cache_read_tokens, 10);
    }

    #[test]
    fn test_record_cumulative_keeps_the_last_total_rather_than_the_sum() {
        let counter = UnifiedTokenCounter::new();
        counter.record_cumulative(&UnifiedUsage {
            output_tokens: 100,
            ..Default::default()
        });
        counter.record_cumulative(&UnifiedUsage {
            output_tokens: 500,
            ..Default::default()
        });

        assert_eq!(
            counter.get_usage().output_tokens,
            500,
            "Anthropic reports running totals, so the last one is the total, not an addend"
        );
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
            h.join()
                .unwrap_or_else(|e| panic!("thread panicked: {e:?}"));
        }
        // After all threads, some value in 0..10 should be set (no crash)
        let _ = counter.get_usage();
    }
}
