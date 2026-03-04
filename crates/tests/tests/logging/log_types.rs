//! LG-01: Request Logging Tests (P1)
//!
//! Tests for verifying router log entry structures and serialization.
//!
//! Key Scenarios:
//! - Router log entries are correctly structured
//! - All fields serialize/deserialize properly
//! - Cost is stored in i64 nanodollars
//! - Token counts are accurate
//! - Timestamps are handled correctly

use burncloud_database_router_log::{DbRouterLog, ModelUsageStats, UsageStats};

/// Test: DbRouterLog default values through serde
#[test]
fn test_router_log_json_roundtrip() {
    let log = DbRouterLog {
        id: 1,
        request_id: "req-12345".to_string(),
        user_id: Some("user-1".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: Some("channel-1".to_string()),
        status_code: 200,
        latency_ms: 150,
        prompt_tokens: 1000,
        completion_tokens: 500,
        cost: 250_000_000, // $0.25 in nanodollars
        created_at: Some("2024-01-15T10:30:00Z".to_string()),
    };

    let json = serde_json::to_string(&log).expect("Should serialize to JSON");
    let deserialized: DbRouterLog =
        serde_json::from_str(&json).expect("Should deserialize from JSON");

    assert_eq!(log.id, deserialized.id);
    assert_eq!(log.request_id, deserialized.request_id);
    assert_eq!(log.user_id, deserialized.user_id);
    assert_eq!(log.path, deserialized.path);
    assert_eq!(log.upstream_id, deserialized.upstream_id);
    assert_eq!(log.status_code, deserialized.status_code);
    assert_eq!(log.latency_ms, deserialized.latency_ms);
    assert_eq!(log.prompt_tokens, deserialized.prompt_tokens);
    assert_eq!(log.completion_tokens, deserialized.completion_tokens);
    assert_eq!(log.cost, deserialized.cost);
}

/// Test: Cost is stored as i64 nanodollars
#[test]
fn test_cost_nanodollar_precision() {
    let log = DbRouterLog {
        id: 1,
        request_id: "req-test".to_string(),
        user_id: None,
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 200,
        latency_ms: 100,
        prompt_tokens: 100,
        completion_tokens: 50,
        // $0.002 = 2,000,000 nanodollars
        cost: 2_000_000,
        created_at: None,
    };

    // Verify the cost is correctly stored as i64
    assert_eq!(log.cost, 2_000_000i64);

    // Verify no floating point issues
    let cost_in_dollars = log.cost as f64 / 1_000_000_000.0;
    assert!((cost_in_dollars - 0.002).abs() < f64::EPSILON);
}

/// Test: High cost values (no overflow)
#[test]
fn test_high_cost_values() {
    // Test with a high cost value (e.g., $1000 = 1,000,000,000,000 nanodollars)
    let high_cost: i64 = 1_000_000_000_000;

    let log = DbRouterLog {
        id: 1,
        request_id: "req-high".to_string(),
        user_id: Some("user-1".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: Some("channel-1".to_string()),
        status_code: 200,
        latency_ms: 5000,
        prompt_tokens: 100000,
        completion_tokens: 50000,
        cost: high_cost,
        created_at: None,
    };

    assert_eq!(log.cost, high_cost);
    assert!(log.cost > 0, "Cost should be positive");
}

/// Test: Token counts are accurate
#[test]
fn test_token_counts() {
    let log = DbRouterLog {
        id: 1,
        request_id: "req-tokens".to_string(),
        user_id: None,
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 200,
        latency_ms: 100,
        prompt_tokens: 1500,
        completion_tokens: 750,
        cost: 0,
        created_at: None,
    };

    let total_tokens = log.prompt_tokens + log.completion_tokens;
    assert_eq!(total_tokens, 2250, "Total tokens should be 2250");
}

/// Test: Status code ranges
#[test]
fn test_status_codes() {
    let success_log = DbRouterLog {
        id: 1,
        request_id: "req-200".to_string(),
        user_id: None,
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 200,
        latency_ms: 100,
        prompt_tokens: 100,
        completion_tokens: 50,
        cost: 0,
        created_at: None,
    };

    let error_log = DbRouterLog {
        id: 2,
        request_id: "req-500".to_string(),
        user_id: None,
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 500,
        latency_ms: 50,
        prompt_tokens: 100,
        completion_tokens: 0,
        cost: 0,
        created_at: None,
    };

    let rate_limit_log = DbRouterLog {
        id: 3,
        request_id: "req-429".to_string(),
        user_id: None,
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 429,
        latency_ms: 10,
        prompt_tokens: 100,
        completion_tokens: 0,
        cost: 0,
        created_at: None,
    };

    // 2xx = success
    assert!(success_log.status_code >= 200 && success_log.status_code < 300);
    // 4xx = client error
    assert!(rate_limit_log.status_code >= 400 && rate_limit_log.status_code < 500);
    // 5xx = server error
    assert!(error_log.status_code >= 500 && error_log.status_code < 600);
}

/// Test: Latency in milliseconds
#[test]
fn test_latency_storage() {
    let log = DbRouterLog {
        id: 1,
        request_id: "req-latency".to_string(),
        user_id: None,
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 200,
        latency_ms: 1234, // 1.234 seconds
        prompt_tokens: 100,
        completion_tokens: 50,
        cost: 0,
        created_at: None,
    };

    assert_eq!(log.latency_ms, 1234);
    // Verify conversion to seconds
    let latency_seconds = log.latency_ms as f64 / 1000.0;
    assert!((latency_seconds - 1.234).abs() < f64::EPSILON);
}

/// Test: Optional user_id field
#[test]
fn test_optional_user_id() {
    // With user_id
    let with_user = DbRouterLog {
        id: 1,
        request_id: "req-1".to_string(),
        user_id: Some("user-123".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 200,
        latency_ms: 100,
        prompt_tokens: 100,
        completion_tokens: 50,
        cost: 0,
        created_at: None,
    };
    assert!(with_user.user_id.is_some());
    assert_eq!(with_user.user_id.unwrap(), "user-123");

    // Without user_id (anonymous/unauthenticated request)
    let without_user = DbRouterLog {
        id: 2,
        request_id: "req-2".to_string(),
        user_id: None,
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 401,
        latency_ms: 5,
        prompt_tokens: 0,
        completion_tokens: 0,
        cost: 0,
        created_at: None,
    };
    assert!(without_user.user_id.is_none());
}

/// Test: Optional upstream_id field
#[test]
fn test_optional_upstream_id() {
    // With upstream_id (successful routing)
    let with_upstream = DbRouterLog {
        id: 1,
        request_id: "req-1".to_string(),
        user_id: Some("user-1".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: Some("channel-openai".to_string()),
        status_code: 200,
        latency_ms: 100,
        prompt_tokens: 100,
        completion_tokens: 50,
        cost: 0,
        created_at: None,
    };
    assert!(with_upstream.upstream_id.is_some());

    // Without upstream_id (failed before routing)
    let without_upstream = DbRouterLog {
        id: 2,
        request_id: "req-2".to_string(),
        user_id: Some("user-1".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 503,
        latency_ms: 10,
        prompt_tokens: 0,
        completion_tokens: 0,
        cost: 0,
        created_at: None,
    };
    assert!(without_upstream.upstream_id.is_none());
}

/// Test: UsageStats default values
#[test]
fn test_usage_stats_default() {
    let stats = UsageStats::default();

    assert_eq!(stats.total_requests, 0);
    assert_eq!(stats.total_prompt_tokens, 0);
    assert_eq!(stats.total_completion_tokens, 0);
    assert_eq!(stats.total_cost_nano, 0);
}

/// Test: UsageStats serialization
#[test]
fn test_usage_stats_serialization() {
    let stats = UsageStats {
        total_requests: 100,
        total_prompt_tokens: 50000,
        total_completion_tokens: 25000,
        total_cost_nano: 750_000_000, // $0.75
    };

    let json = serde_json::to_string(&stats).expect("Should serialize");
    let deserialized: UsageStats = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(stats.total_requests, deserialized.total_requests);
    assert_eq!(stats.total_prompt_tokens, deserialized.total_prompt_tokens);
    assert_eq!(
        stats.total_completion_tokens,
        deserialized.total_completion_tokens
    );
    assert_eq!(stats.total_cost_nano, deserialized.total_cost_nano);
}

/// Test: ModelUsageStats structure
#[test]
fn test_model_usage_stats() {
    let stats = ModelUsageStats {
        model: "gpt-4".to_string(),
        requests: 50,
        prompt_tokens: 25000,
        completion_tokens: 12500,
        cost_nano: 1_500_000_000, // $1.50
    };

    assert_eq!(stats.model, "gpt-4");
    assert_eq!(stats.requests, 50);
    assert_eq!(stats.prompt_tokens, 25000);
    assert_eq!(stats.completion_tokens, 12500);
    assert_eq!(stats.cost_nano, 1_500_000_000);
}

/// Test: ModelUsageStats serialization
#[test]
fn test_model_usage_stats_serialization() {
    let stats = ModelUsageStats {
        model: "claude-3-opus".to_string(),
        requests: 25,
        prompt_tokens: 10000,
        completion_tokens: 5000,
        cost_nano: 500_000_000, // $0.50
    };

    let json = serde_json::to_string(&stats).expect("Should serialize");
    let deserialized: ModelUsageStats = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(stats.model, deserialized.model);
    assert_eq!(stats.requests, deserialized.requests);
    assert_eq!(stats.prompt_tokens, deserialized.prompt_tokens);
    assert_eq!(stats.completion_tokens, deserialized.completion_tokens);
    assert_eq!(stats.cost_nano, deserialized.cost_nano);
}

/// Test: Real-world log entry pattern
#[test]
fn test_real_world_log_entry() {
    // Simulate a real GPT-4 request
    let log = DbRouterLog {
        id: 12345,
        request_id: "req_live_abc123".to_string(),
        user_id: Some("42".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: Some("ch_openai_premium".to_string()),
        status_code: 200,
        latency_ms: 2345, // 2.345 seconds
        prompt_tokens: 1500,
        completion_tokens: 800,
        // GPT-4 pricing: $0.03/1K prompt, $0.06/1K completion
        // 1500 prompt = $0.045, 800 completion = $0.048
        // Total: $0.093 = 93,000,000 nanodollars
        cost: 93_000_000,
        created_at: Some("2024-01-15T14:30:00Z".to_string()),
    };

    // Verify all fields
    assert_eq!(log.id, 12345);
    assert_eq!(log.request_id, "req_live_abc123");
    assert_eq!(log.user_id, Some("42".to_string()));
    assert_eq!(log.path, "/v1/chat/completions");
    assert_eq!(log.upstream_id, Some("ch_openai_premium".to_string()));
    assert_eq!(log.status_code, 200);
    assert_eq!(log.latency_ms, 2345);
    assert_eq!(log.prompt_tokens, 1500);
    assert_eq!(log.completion_tokens, 800);
    assert_eq!(log.cost, 93_000_000);
}

/// Test: Edge case - zero tokens
#[test]
fn test_zero_token_log() {
    let log = DbRouterLog {
        id: 1,
        request_id: "req-zero".to_string(),
        user_id: None,
        path: "/v1/chat/completions".to_string(),
        upstream_id: None,
        status_code: 401, // Unauthorized
        latency_ms: 5,
        prompt_tokens: 0,
        completion_tokens: 0,
        cost: 0,
        created_at: None,
    };

    assert_eq!(log.prompt_tokens, 0);
    assert_eq!(log.completion_tokens, 0);
    assert_eq!(log.cost, 0);
    assert_eq!(log.prompt_tokens + log.completion_tokens, 0);
}

/// Test: Edge case - very large token counts
#[test]
fn test_large_token_counts() {
    let log = DbRouterLog {
        id: 1,
        request_id: "req-large".to_string(),
        user_id: Some("user-1".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: Some("channel-1".to_string()),
        status_code: 200,
        latency_ms: 30000, // 30 seconds
        prompt_tokens: 100000,  // 100K tokens
        completion_tokens: 50000, // 50K tokens
        cost: 10_000_000_000, // $10.00
        created_at: None,
    };

    // Verify no overflow in addition
    let total = log.prompt_tokens + log.completion_tokens;
    assert_eq!(total, 150000);
}
