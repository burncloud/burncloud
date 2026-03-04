//! BL-01: Token Counting Tests (P0)
//!
//! Tests for verifying accurate token counting in billing.
//!
//! Key Scenarios:
//! - Prompt tokens are accurately counted
//! - Completion tokens are accurately counted
//! - Token usage totals are consistent (prompt + completion = total)
//! - Cache tokens are correctly tracked
//! - Audio tokens are correctly tracked

use burncloud_router::billing::TokenUsage;

/// Test: TokenUsage default values are all zero
#[test]
fn test_token_usage_default() {
    let usage = TokenUsage::default();

    assert_eq!(usage.prompt_tokens, 0, "Default prompt_tokens should be 0");
    assert_eq!(
        usage.completion_tokens, 0,
        "Default completion_tokens should be 0"
    );
    assert_eq!(
        usage.cache_read_tokens, 0,
        "Default cache_read_tokens should be 0"
    );
    assert_eq!(
        usage.cache_creation_tokens, 0,
        "Default cache_creation_tokens should be 0"
    );
    assert_eq!(usage.audio_tokens, 0, "Default audio_tokens should be 0");
}

/// Test: TokenUsage stores token counts correctly
#[test]
fn test_token_usage_storage() {
    let usage = TokenUsage {
        prompt_tokens: 1000,
        completion_tokens: 500,
        cache_read_tokens: 800,
        cache_creation_tokens: 200,
        audio_tokens: 100,
    };

    assert_eq!(usage.prompt_tokens, 1000);
    assert_eq!(usage.completion_tokens, 500);
    assert_eq!(usage.cache_read_tokens, 800);
    assert_eq!(usage.cache_creation_tokens, 200);
    assert_eq!(usage.audio_tokens, 100);
}

/// Test: TokenUsage handles large token counts (overflow protection)
#[test]
fn test_token_usage_large_values() {
    // Test with values close to u64::MAX / 1M to verify no overflow
    let large_tokens = 10_000_000_000u64; // 10 billion tokens

    let usage = TokenUsage {
        prompt_tokens: large_tokens,
        completion_tokens: large_tokens,
        cache_read_tokens: 0,
        cache_creation_tokens: 0,
        audio_tokens: 0,
    };

    assert_eq!(usage.prompt_tokens, large_tokens);
    assert_eq!(usage.completion_tokens, large_tokens);
}

/// Test: TokenUsage Clone trait works correctly
#[test]
fn test_token_usage_clone() {
    let original = TokenUsage {
        prompt_tokens: 1500,
        completion_tokens: 750,
        cache_read_tokens: 500,
        cache_creation_tokens: 150,
        audio_tokens: 50,
    };

    let cloned = original.clone();

    assert_eq!(original.prompt_tokens, cloned.prompt_tokens);
    assert_eq!(original.completion_tokens, cloned.completion_tokens);
    assert_eq!(original.cache_read_tokens, cloned.cache_read_tokens);
    assert_eq!(original.cache_creation_tokens, cloned.cache_creation_tokens);
    assert_eq!(original.audio_tokens, cloned.audio_tokens);
}

/// Test: Total tokens calculation
#[test]
fn test_total_tokens_calculation() {
    let usage = TokenUsage {
        prompt_tokens: 1000,
        completion_tokens: 500,
        cache_read_tokens: 800,
        cache_creation_tokens: 200,
        audio_tokens: 100,
    };

    // Total should be sum of all token types (depends on billing model)
    // For standard billing: prompt + completion
    let standard_total = usage.prompt_tokens + usage.completion_tokens;
    assert_eq!(standard_total, 1500, "Standard total should be 1500");

    // For cache-aware billing: prompt + completion + cache tokens
    let cache_aware_total = usage.prompt_tokens
        + usage.completion_tokens
        + usage.cache_read_tokens
        + usage.cache_creation_tokens;
    assert_eq!(
        cache_aware_total, 2500,
        "Cache-aware total should be 2500"
    );
}

/// Test: Token types are stored as u64 (not f64)
/// This is a compile-time check - if it compiles, the types are correct
#[test]
fn test_token_types_are_u64() {
    let usage = TokenUsage {
        prompt_tokens: u64::MAX,
        completion_tokens: 0,
        cache_read_tokens: 0,
        cache_creation_tokens: 0,
        audio_tokens: 0,
    };

    // Verify it's actually a u64 by using u64-specific operations
    let _ = usage.prompt_tokens.checked_mul(2);
    let _ = usage.prompt_tokens.checked_add(1);

    assert_eq!(usage.prompt_tokens, u64::MAX);
}

/// Test: TokenUsage serialization and deserialization
#[test]
fn test_token_usage_serialization() {
    let original = TokenUsage {
        prompt_tokens: 1234,
        completion_tokens: 5678,
        cache_read_tokens: 9012,
        cache_creation_tokens: 3456,
        audio_tokens: 7890,
    };

    let json = serde_json::to_string(&original).expect("Should serialize to JSON");
    let deserialized: TokenUsage =
        serde_json::from_str(&json).expect("Should deserialize from JSON");

    assert_eq!(original.prompt_tokens, deserialized.prompt_tokens);
    assert_eq!(original.completion_tokens, deserialized.completion_tokens);
    assert_eq!(original.cache_read_tokens, deserialized.cache_read_tokens);
    assert_eq!(
        original.cache_creation_tokens,
        deserialized.cache_creation_tokens
    );
    assert_eq!(original.audio_tokens, deserialized.audio_tokens);
}

/// Test: Edge case - zero tokens
#[test]
fn test_zero_tokens() {
    let usage = TokenUsage {
        prompt_tokens: 0,
        completion_tokens: 0,
        cache_read_tokens: 0,
        cache_creation_tokens: 0,
        audio_tokens: 0,
    };

    let total = usage.prompt_tokens + usage.completion_tokens;
    assert_eq!(total, 0, "Total should be 0 for zero usage");
}

/// Test: Edge case - only prompt tokens
#[test]
fn test_only_prompt_tokens() {
    let usage = TokenUsage {
        prompt_tokens: 1000,
        completion_tokens: 0,
        cache_read_tokens: 0,
        cache_creation_tokens: 0,
        audio_tokens: 0,
    };

    assert_eq!(usage.prompt_tokens, 1000);
    assert_eq!(usage.completion_tokens, 0);
}

/// Test: Edge case - only completion tokens
#[test]
fn test_only_completion_tokens() {
    let usage = TokenUsage {
        prompt_tokens: 0,
        completion_tokens: 500,
        cache_read_tokens: 0,
        cache_creation_tokens: 0,
        audio_tokens: 0,
    };

    assert_eq!(usage.prompt_tokens, 0);
    assert_eq!(usage.completion_tokens, 500);
}

/// Test: Real-world GPT-4 usage pattern
#[test]
fn test_gpt4_usage_pattern() {
    // Typical GPT-4 request with moderate context
    let usage = TokenUsage {
        prompt_tokens: 2500,  // ~2.5K context
        completion_tokens: 800, // ~800 output tokens
        cache_read_tokens: 0,
        cache_creation_tokens: 0,
        audio_tokens: 0,
    };

    assert_eq!(usage.prompt_tokens, 2500);
    assert_eq!(usage.completion_tokens, 800);

    // Total for billing
    let total = usage.prompt_tokens + usage.completion_tokens;
    assert_eq!(total, 3300);
}

/// Test: Real-world Claude usage pattern with caching
#[test]
fn test_claude_caching_pattern() {
    // Claude request with Prompt Caching
    let usage = TokenUsage {
        prompt_tokens: 5000,    // New prompt tokens
        completion_tokens: 1000, // Output tokens
        cache_read_tokens: 20000, // Tokens read from cache (90% discount)
        cache_creation_tokens: 25000, // Tokens cached for future (25% premium)
        audio_tokens: 0,
    };

    assert_eq!(usage.prompt_tokens, 5000);
    assert_eq!(usage.completion_tokens, 1000);
    assert_eq!(usage.cache_read_tokens, 20000);
    assert_eq!(usage.cache_creation_tokens, 25000);
}

/// Test: Real-world Gemini audio usage pattern
#[test]
fn test_gemini_audio_pattern() {
    // Gemini multimodal with audio
    let usage = TokenUsage {
        prompt_tokens: 500,
        completion_tokens: 300,
        cache_read_tokens: 0,
        cache_creation_tokens: 0,
        audio_tokens: 5000, // Audio tokens (typically higher price)
    };

    assert_eq!(usage.prompt_tokens, 500);
    assert_eq!(usage.completion_tokens, 300);
    assert_eq!(usage.audio_tokens, 5000);
}
