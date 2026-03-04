//! LG-03: User Usage Statistics Tests (P1)
//!
//! Tests for verifying token aggregation and usage statistics.
//!
//! Key Scenarios:
//! - Prompt/Completion token aggregation
//! - Cost calculation in nanodollars
//! - Usage stats grouping by model
//! - Time period filtering (day/week/month)
//! - Multiple request accumulation

use burncloud_database_router_log::{ModelUsageStats, UsageStats};

/// Test: UsageStats accumulation calculation
#[test]
fn test_usage_stats_calculation() {
    // Simulate multiple log entries being aggregated
    let logs = vec![
        (1000, 500, 100_000_000i64),  // Log 1
        (2000, 1000, 250_000_000i64), // Log 2
        (500, 250, 50_000_000i64),    // Log 3
    ];

    let total_prompt: i64 = logs.iter().map(|(p, _, _)| *p as i64).sum();
    let total_completion: i64 = logs.iter().map(|(_, c, _)| *c as i64).sum();
    let total_cost: i64 = logs.iter().map(|(_, _, c)| *c).sum();

    let stats = UsageStats {
        total_requests: logs.len() as i64,
        total_prompt_tokens: total_prompt,
        total_completion_tokens: total_completion,
        total_cost_nano: total_cost,
    };

    assert_eq!(stats.total_requests, 3);
    assert_eq!(stats.total_prompt_tokens, 3500);
    assert_eq!(stats.total_completion_tokens, 1750);
    assert_eq!(stats.total_cost_nano, 400_000_000); // $0.40
}

/// Test: UsageStats total tokens
#[test]
fn test_usage_stats_total_tokens() {
    let stats = UsageStats {
        total_requests: 100,
        total_prompt_tokens: 50000,
        total_completion_tokens: 25000,
        total_cost_nano: 750_000_000,
    };

    let total_tokens = stats.total_prompt_tokens + stats.total_completion_tokens;
    assert_eq!(total_tokens, 75000);
}

/// Test: UsageStats cost conversion
#[test]
fn test_usage_stats_cost_conversion() {
    let stats = UsageStats {
        total_requests: 10,
        total_prompt_tokens: 10000,
        total_completion_tokens: 5000,
        total_cost_nano: 150_000_000, // $0.15
    };

    // Convert nanodollars to dollars
    let cost_dollars = stats.total_cost_nano as f64 / 1_000_000_000.0;
    assert!((cost_dollars - 0.15).abs() < f64::EPSILON);

    // Convert nanodollars to cents
    let cost_cents = stats.total_cost_nano / 10_000_000;
    assert_eq!(cost_cents, 15);
}

/// Test: ModelUsageStats aggregation
#[test]
fn test_model_usage_stats_aggregation() {
    // Simulate logs for different models
    let model_logs = vec![
        ("gpt-4".to_string(), 1500, 800, 93_000_000i64),
        ("gpt-4".to_string(), 2000, 1000, 120_000_000i64),
        ("gpt-3.5-turbo".to_string(), 1000, 500, 5_000_000i64),
        ("gpt-3.5-turbo".to_string(), 800, 400, 4_000_000i64),
    ];

    // Group by model
    let mut model_stats: std::collections::HashMap<String, (i64, i64, i64, i64)> =
        std::collections::HashMap::new();

    for (model, prompt, completion, cost) in model_logs {
        let entry = model_stats.entry(model).or_insert((0, 0, 0, 0));
        entry.0 += 1; // requests
        entry.1 += prompt as i64;
        entry.2 += completion as i64;
        entry.3 += cost;
    }

    // Verify GPT-4 stats
    let gpt4 = model_stats.get("gpt-4").unwrap();
    assert_eq!(gpt4.0, 2); // 2 requests
    assert_eq!(gpt4.1, 3500); // total prompt tokens
    assert_eq!(gpt4.2, 1800); // total completion tokens
    assert_eq!(gpt4.3, 213_000_000); // $0.213

    // Verify GPT-3.5 stats
    let gpt35 = model_stats.get("gpt-3.5-turbo").unwrap();
    assert_eq!(gpt35.0, 2); // 2 requests
    assert_eq!(gpt35.1, 1800); // total prompt tokens
    assert_eq!(gpt35.2, 900); // total completion tokens
    assert_eq!(gpt35.3, 9_000_000); // $0.009
}

/// Test: Empty usage stats
#[test]
fn test_empty_usage_stats() {
    let stats = UsageStats::default();

    assert_eq!(stats.total_requests, 0);
    assert_eq!(stats.total_prompt_tokens, 0);
    assert_eq!(stats.total_completion_tokens, 0);
    assert_eq!(stats.total_cost_nano, 0);
}

/// Test: High volume usage stats
#[test]
fn test_high_volume_usage_stats() {
    // Simulate a power user with high usage
    let stats = UsageStats {
        total_requests: 10000,
        total_prompt_tokens: 50_000_000,   // 50M tokens
        total_completion_tokens: 25_000_000, // 25M tokens
        total_cost_nano: 750_000_000_000,  // $750.00
    };

    assert_eq!(stats.total_requests, 10000);
    assert_eq!(stats.total_prompt_tokens, 50_000_000);
    assert_eq!(stats.total_completion_tokens, 25_000_000);
    assert_eq!(stats.total_cost_nano, 750_000_000_000);

    // Verify cost per request
    let cost_per_request = stats.total_cost_nano / stats.total_requests;
    assert_eq!(cost_per_request, 75_000_000); // $0.075 per request

    // Verify tokens per request
    let tokens_per_request = (stats.total_prompt_tokens + stats.total_completion_tokens)
        / stats.total_requests;
    assert_eq!(tokens_per_request, 7500);
}

/// Test: ModelUsageStats JSON serialization
#[test]
fn test_model_usage_stats_json() {
    let stats = ModelUsageStats {
        model: "gpt-4-turbo".to_string(),
        requests: 100,
        prompt_tokens: 50000,
        completion_tokens: 25000,
        cost_nano: 2_500_000_000, // $2.50
    };

    let json = serde_json::to_string(&stats).unwrap();
    assert!(json.contains("\"model\":\"gpt-4-turbo\""));
    assert!(json.contains("\"requests\":100"));
    assert!(json.contains("\"prompt_tokens\":50000"));
    assert!(json.contains("\"completion_tokens\":25000"));
    assert!(json.contains("\"cost_nano\":2500000000"));
}

/// Test: UsageStats JSON serialization
#[test]
fn test_usage_stats_json() {
    let stats = UsageStats {
        total_requests: 500,
        total_prompt_tokens: 250000,
        total_completion_tokens: 125000,
        total_cost_nano: 3_750_000_000, // $3.75
    };

    let json = serde_json::to_string(&stats).unwrap();
    assert!(json.contains("\"total_requests\":500"));
    assert!(json.contains("\"total_prompt_tokens\":250000"));
    assert!(json.contains("\"total_completion_tokens\":125000"));
    assert!(json.contains("\"total_cost_nano\":3750000000"));
}

/// Test: Cost calculation formula verification
#[test]
fn test_cost_calculation_formula() {
    // GPT-4 pricing: $0.03/1K prompt, $0.06/1K completion
    let prompt_price_per_million = 30_000_000_000i64; // $30 per million in nanodollars
    let completion_price_per_million = 60_000_000_000i64; // $60 per million in nanodollars

    let prompt_tokens = 1500i64;
    let completion_tokens = 800i64;

    // Calculate cost: (prompt_tokens / 1M) * prompt_price + (completion_tokens / 1M) * completion_price
    let prompt_cost = (prompt_tokens * prompt_price_per_million) / 1_000_000;
    let completion_cost = (completion_tokens * completion_price_per_million) / 1_000_000;
    let total_cost = prompt_cost + completion_cost;

    // Expected: (1500 * 30B / 1M) + (800 * 60B / 1M) = 45,000,000 + 48,000,000 = 93,000,000
    assert_eq!(prompt_cost, 45_000_000); // $0.045
    assert_eq!(completion_cost, 48_000_000); // $0.048
    assert_eq!(total_cost, 93_000_000); // $0.093
}

/// Test: Time period concept (day/week/month)
#[test]
fn test_time_period_calculation() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Day: 24 hours
    let day_threshold = now - (24 * 60 * 60);
    assert!(day_threshold < now);

    // Week: 7 days
    let week_threshold = now - (7 * 24 * 60 * 60);
    assert!(week_threshold < day_threshold);

    // Month: 30 days
    let month_threshold = now - (30 * 24 * 60 * 60);
    assert!(month_threshold < week_threshold);
}

/// Test: Usage percentage calculation
#[test]
fn test_usage_percentage() {
    let used_tokens = 75000i64;
    let quota_tokens = 100000i64;

    let usage_percent = (used_tokens as f64 / quota_tokens as f64) * 100.0;
    assert!((usage_percent - 75.0).abs() < f64::EPSILON);
}

/// Test: Average latency calculation
#[test]
fn test_average_latency_calculation() {
    let latencies = vec![100, 150, 200, 125, 175]; // milliseconds
    let avg_latency = latencies.iter().sum::<i64>() as f64 / latencies.len() as f64;

    assert!((avg_latency - 150.0).abs() < f64::EPSILON);
}

/// Test: Request rate calculation
#[test]
fn test_request_rate_calculation() {
    let total_requests = 1000i64;
    let time_period_seconds = 3600i64; // 1 hour

    let requests_per_second = total_requests as f64 / time_period_seconds as f64;
    assert!((requests_per_second - (1000.0 / 3600.0)).abs() < f64::EPSILON);

    let requests_per_minute = total_requests as f64 / (time_period_seconds as f64 / 60.0);
    assert!((requests_per_minute - (1000.0 / 60.0)).abs() < f64::EPSILON);
}

/// Test: Token efficiency ratio
#[test]
fn test_token_efficiency_ratio() {
    // How efficiently are prompt tokens being converted to completion tokens?
    let total_prompt = 10000i64;
    let total_completion = 5000i64;

    let efficiency_ratio = total_completion as f64 / total_prompt as f64;
    assert!((efficiency_ratio - 0.5).abs() < f64::EPSILON);
}

/// Test: Cost per token calculation
#[test]
fn test_cost_per_token() {
    let total_cost = 93_000_000i64; // $0.093
    let total_tokens = 2300i64; // 1500 prompt + 800 completion

    let cost_per_token_nano = total_cost / total_tokens;
    let cost_per_token_dollar = cost_per_token_nano as f64 / 1_000_000_000.0;

    // $0.093 / 2300 = $0.0000404 per token
    assert!(cost_per_token_dollar > 0.0);
    assert!(cost_per_token_dollar < 0.001); // Less than 0.1 cent per token
}

/// Test: Multiple model comparison
#[test]
fn test_multiple_model_comparison() {
    let models = vec![
        ModelUsageStats {
            model: "gpt-4".to_string(),
            requests: 100,
            prompt_tokens: 50000,
            completion_tokens: 25000,
            cost_nano: 2_250_000_000, // $2.25
        },
        ModelUsageStats {
            model: "gpt-3.5-turbo".to_string(),
            requests: 500,
            prompt_tokens: 250000,
            completion_tokens: 125000,
            cost_nano: 375_000_000, // $0.375
        },
        ModelUsageStats {
            model: "claude-3-opus".to_string(),
            requests: 50,
            prompt_tokens: 25000,
            completion_tokens: 12500,
            cost_nano: 1_875_000_000, // $1.875
        },
    ];

    // Find the most expensive model
    let most_expensive = models.iter().max_by_key(|m| m.cost_nano).unwrap();
    assert_eq!(most_expensive.model, "gpt-4");

    // Find the most used model (by requests)
    let most_used = models.iter().max_by_key(|m| m.requests).unwrap();
    assert_eq!(most_used.model, "gpt-3.5-turbo");

    // Calculate total across all models
    let total_cost: i64 = models.iter().map(|m| m.cost_nano).sum();
    assert_eq!(total_cost, 4_500_000_000); // $4.50
}
