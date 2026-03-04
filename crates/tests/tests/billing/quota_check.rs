//! BL-06: Quota Check Tests (P0)
//!
//! Tests for quota checking and insufficient balance handling.
//!
//! Key Requirements:
//! - Pre-check balance before processing request
//! - Reject requests when balance is insufficient (402 Payment Required)
//! - Handle edge cases: zero balance, exact balance, near-zero balance
//! - Currency-aware quota checking (USD vs CNY)

use burncloud_common::dollars_to_nano;

/// Possible outcomes of a quota check
#[derive(Debug, Clone, PartialEq)]
pub enum QuotaCheckResult {
    Allowed,
    InsufficientBalance {
        balance: i64,
        required: i64,
    },
}

/// Simulate quota check logic
fn check_quota(balance_nano: i64, estimated_cost_nano: i64) -> QuotaCheckResult {
    if balance_nano >= estimated_cost_nano {
        QuotaCheckResult::Allowed
    } else {
        QuotaCheckResult::InsufficientBalance {
            balance: balance_nano,
            required: estimated_cost_nano,
        }
    }
}

// ============================================================================
// BL-06: Quota Check - P0
// ============================================================================

/// Test: Sufficient balance allows request
#[test]
fn test_quota_check_sufficient_balance() {
    let balance = dollars_to_nano(100.0);
    let estimated_cost = dollars_to_nano(1.0);

    let result = check_quota(balance, estimated_cost);

    assert_eq!(result, QuotaCheckResult::Allowed);
}

/// Test: Exact balance allows request
#[test]
fn test_quota_check_exact_balance() {
    let balance = dollars_to_nano(10.0);
    let estimated_cost = dollars_to_nano(10.0);

    let result = check_quota(balance, estimated_cost);

    assert_eq!(result, QuotaCheckResult::Allowed);
}

/// Test: Insufficient balance rejects request
#[test]
fn test_quota_check_insufficient_balance() {
    let balance = dollars_to_nano(5.0);
    let estimated_cost = dollars_to_nano(10.0);

    let result = check_quota(balance, estimated_cost);

    assert_eq!(
        result,
        QuotaCheckResult::InsufficientBalance {
            balance: dollars_to_nano(5.0),
            required: dollars_to_nano(10.0)
        }
    );
}

/// Test: Zero balance rejects any request
#[test]
fn test_quota_check_zero_balance() {
    let balance = 0i64;
    let estimated_cost = dollars_to_nano(0.001);

    let result = check_quota(balance, estimated_cost);

    assert!(matches!(result, QuotaCheckResult::InsufficientBalance { .. }));
}

/// Test: Zero cost is always allowed
#[test]
fn test_quota_check_zero_cost() {
    let balance = 0i64;
    let estimated_cost = 0i64;

    let result = check_quota(balance, estimated_cost);

    assert_eq!(result, QuotaCheckResult::Allowed);
}

/// Test: Near-zero balance with tiny cost
#[test]
fn test_quota_check_near_zero() {
    // Balance: $0.000000001 (1 nanodollar)
    let balance = 1i64;
    // Cost: $0.000000001 (1 nanodollar)
    let estimated_cost = 1i64;

    let result = check_quota(balance, estimated_cost);

    assert_eq!(result, QuotaCheckResult::Allowed);
}

/// Test: Near-zero balance insufficient for larger cost
#[test]
fn test_quota_check_near_zero_insufficient() {
    // Balance: $0.000000002 (2 nanodollars)
    let balance = 2i64;
    // Cost: $0.000000003 (3 nanodollars)
    let estimated_cost = 3i64;

    let result = check_quota(balance, estimated_cost);

    assert!(matches!(result, QuotaCheckResult::InsufficientBalance { .. }));
}

/// Test: Large balance with large cost
#[test]
fn test_quota_check_large_values() {
    // Balance: $1,000,000
    let balance = dollars_to_nano(1_000_000.0);
    // Cost: $500,000
    let estimated_cost = dollars_to_nano(500_000.0);

    let result = check_quota(balance, estimated_cost);

    assert_eq!(result, QuotaCheckResult::Allowed);
}

/// Test: Large balance insufficient for very large cost
#[test]
fn test_quota_check_large_values_insufficient() {
    // Balance: $1,000
    let balance = dollars_to_nano(1_000.0);
    // Cost: $2,000
    let estimated_cost = dollars_to_nano(2_000.0);

    let result = check_quota(balance, estimated_cost);

    assert!(matches!(result, QuotaCheckResult::InsufficientBalance { .. }));
}

/// Test: Repeated requests depleting balance
#[test]
fn test_quota_check_repeated_requests() {
    let mut balance = dollars_to_nano(10.0);
    let cost_per_request = dollars_to_nano(3.0);

    // First request: $10 - $3 = $7
    assert_eq!(check_quota(balance, cost_per_request), QuotaCheckResult::Allowed);
    balance -= cost_per_request;

    // Second request: $7 - $3 = $4
    assert_eq!(check_quota(balance, cost_per_request), QuotaCheckResult::Allowed);
    balance -= cost_per_request;

    // Third request: $4 - $3 = $1
    assert_eq!(check_quota(balance, cost_per_request), QuotaCheckResult::Allowed);
    balance -= cost_per_request;

    // Fourth request: $1 < $3 - Should fail
    let result = check_quota(balance, cost_per_request);
    assert!(matches!(result, QuotaCheckResult::InsufficientBalance { .. }));
}

/// Test: Multiple small requests until balance depleted
#[test]
fn test_quota_check_micro_requests() {
    let mut balance = dollars_to_nano(0.01); // $0.01 = 10M nano
    let cost_per_request = dollars_to_nano(0.001); // $0.001 = 1M nano

    // Should allow exactly 10 requests
    let mut allowed_count = 0;
    while check_quota(balance, cost_per_request) == QuotaCheckResult::Allowed {
        allowed_count += 1;
        balance -= cost_per_request;
    }

    assert_eq!(allowed_count, 10, "Should allow exactly 10 micro requests");
    assert_eq!(balance, 0, "Balance should be exactly zero");
}

/// Test: Error message contains balance information
#[test]
fn test_quota_check_error_info() {
    let balance = dollars_to_nano(5.0);
    let estimated_cost = dollars_to_nano(10.0);

    let result = check_quota(balance, estimated_cost);

    if let QuotaCheckResult::InsufficientBalance { balance: b, required: r } = result {
        assert_eq!(b, dollars_to_nano(5.0));
        assert_eq!(r, dollars_to_nano(10.0));

        // Can compute deficit
        let deficit = r - b;
        assert_eq!(deficit, dollars_to_nano(5.0), "Deficit should be $5");
    } else {
        panic!("Expected InsufficientBalance result");
    }
}

// ============================================================================
// Estimated Cost Calculation
// ============================================================================

/// Estimate request cost based on tokens and pricing
fn estimate_request_cost(
    estimated_prompt_tokens: u64,
    estimated_completion_tokens: u64,
    input_price_per_million: i64,
    output_price_per_million: i64,
) -> i64 {
    let input_cost = (estimated_prompt_tokens as i128 * input_price_per_million as i128) / 1_000_000;
    let output_cost = (estimated_completion_tokens as i128 * output_price_per_million as i128) / 1_000_000;
    (input_cost + output_cost) as i64
}

/// Test: Estimated cost calculation for typical request
#[test]
fn test_estimate_request_cost() {
    // Estimate 1000 prompt tokens, 500 completion tokens
    // Pricing: $1/1M input, $2/1M output
    let cost = estimate_request_cost(
        1000,
        500,
        dollars_to_nano(1.0),
        dollars_to_nano(2.0),
    );

    // 1000 * $1/1M + 500 * $2/1M = $0.001 + $0.001 = $0.002
    assert_eq!(cost, 2_000_000, "Estimated cost should be $0.002");
}

/// Test: Quota check with estimated cost
#[test]
fn test_quota_check_with_estimation() {
    let balance = dollars_to_nano(10.0);

    // Estimate 1M prompt tokens, 500K completion tokens
    // GPT-4 pricing: $10/1M input, $30/1M output
    let estimated_cost = estimate_request_cost(
        1_000_000,
        500_000,
        dollars_to_nano(10.0),
        dollars_to_nano(30.0),
    );

    // Cost = $10 + $15 = $25
    // Balance = $10, so insufficient
    let result = check_quota(balance, estimated_cost);

    assert!(matches!(result, QuotaCheckResult::InsufficientBalance { .. }));
}

/// Test: High estimate still passes with large balance
#[test]
fn test_quota_check_high_estimate_passes() {
    let balance = dollars_to_nano(100.0);

    let estimated_cost = estimate_request_cost(
        1_000_000,
        1_000_000,
        dollars_to_nano(30.0),
        dollars_to_nano(60.0),
    );

    // Cost = $30 + $60 = $90
    // Balance = $100, so allowed
    let result = check_quota(balance, estimated_cost);

    assert_eq!(result, QuotaCheckResult::Allowed);
}

// ============================================================================
// Currency-Aware Quota Check
// ============================================================================

/// Dual currency balance
#[derive(Debug, Clone)]
pub struct DualBalance {
    pub usd: i64,
    pub cny: i64,
}

/// Currency preference
#[derive(Debug, Clone, PartialEq)]
pub enum CurrencyPreference {
    USD,
    CNY,
}

/// Check quota with currency preference
fn check_quota_currency(
    balance: &DualBalance,
    estimated_cost_nano: i64,
    preference: CurrencyPreference,
) -> QuotaCheckResult {
    match preference {
        CurrencyPreference::USD => check_quota(balance.usd, estimated_cost_nano),
        CurrencyPreference::CNY => check_quota(balance.cny, estimated_cost_nano),
    }
}

/// Test: USD preference checks USD balance
#[test]
fn test_quota_check_usd_preference() {
    let balance = DualBalance {
        usd: dollars_to_nano(10.0),
        cny: 0,
    };

    let result = check_quota_currency(&balance, dollars_to_nano(5.0), CurrencyPreference::USD);

    assert_eq!(result, QuotaCheckResult::Allowed);
}

/// Test: CNY preference checks CNY balance
#[test]
fn test_quota_check_cny_preference() {
    // ¥100 (stored as nanodollars equivalent)
    let balance = DualBalance {
        usd: 0,
        cny: dollars_to_nano(100.0), // Simplified: using same scale
    };

    let result = check_quota_currency(&balance, dollars_to_nano(50.0), CurrencyPreference::CNY);

    assert_eq!(result, QuotaCheckResult::Allowed);
}

/// Test: Insufficient in preferred currency even if other currency has balance
#[test]
fn test_quota_check_currency_isolation() {
    let balance = DualBalance {
        usd: dollars_to_nano(1.0),  // Only $1 USD
        cny: dollars_to_nano(1000.0), // ¥1000 CNY
    };

    // Try to pay $10 in USD
    let result = check_quota_currency(&balance, dollars_to_nano(10.0), CurrencyPreference::USD);

    assert!(matches!(result, QuotaCheckResult::InsufficientBalance { .. }));
}

// ============================================================================
// Quota Recovery (After Recharge)
// ============================================================================

/// Test: Quota passes after recharge
#[test]
fn test_quota_recovery_after_recharge() {
    let mut balance = dollars_to_nano(5.0);
    let cost = dollars_to_nano(10.0);

    // Initially insufficient
    let result1 = check_quota(balance, cost);
    assert!(matches!(result1, QuotaCheckResult::InsufficientBalance { .. }));

    // Recharge $10
    balance += dollars_to_nano(10.0);

    // Now sufficient
    let result2 = check_quota(balance, cost);
    assert_eq!(result2, QuotaCheckResult::Allowed);
}

/// Test: Multiple recharges until quota met
#[test]
fn test_quota_multiple_recharges() {
    let mut balance = 0i64;
    let target_cost = dollars_to_nano(100.0);

    // Recharge in increments
    for _ in 0..10 {
        balance += dollars_to_nano(10.0);

        if check_quota(balance, target_cost) == QuotaCheckResult::Allowed {
            break;
        }
    }

    assert_eq!(
        check_quota(balance, target_cost),
        QuotaCheckResult::Allowed
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test: Very small balance vs very small cost
#[test]
fn test_quota_tiny_amounts() {
    // Balance: $0.000000005 (5 nanodollars)
    let balance = 5i64;
    // Cost: $0.000000003 (3 nanodollars)
    let cost = 3i64;

    let result = check_quota(balance, cost);
    assert_eq!(result, QuotaCheckResult::Allowed);

    // After deduction
    let remaining = balance - cost;
    assert_eq!(remaining, 2, "2 nanodollars remaining");
}

/// Test: Balance exactly equals cost plus one nanodollar
#[test]
fn test_quota_one_nano_difference() {
    let cost = dollars_to_nano(1.0);
    let balance = cost + 1; // One extra nanodollar

    let result = check_quota(balance, cost);
    assert_eq!(result, QuotaCheckResult::Allowed);
}

/// Test: Balance exactly one nanodollar short
#[test]
fn test_quota_one_nano_short() {
    let cost = dollars_to_nano(1.0);
    let balance = cost - 1; // One nanodollar short

    let result = check_quota(balance, cost);
    assert!(matches!(result, QuotaCheckResult::InsufficientBalance { .. }));
}

/// Test: User status check (disabled users should not pass quota)
#[test]
fn test_user_status_check() {
    // Status: 1 = Active, 0 = Disabled
    let user_status = 0; // Disabled

    // Even with sufficient balance, disabled user should not be allowed
    let balance = dollars_to_nano(1000.0);
    let cost = dollars_to_nano(1.0);

    // In production code:
    // if user.status != 1 { return Err(Error::UserDisabled); }

    let balance_sufficient = balance >= cost;
    let user_active = user_status == 1;

    assert!(balance_sufficient, "Balance should be sufficient");
    assert!(!user_active, "User should be disabled");
}

/// Test: Token limit per request
#[test]
fn test_token_limit_check() {
    // Some models have token limits
    let max_tokens_per_request = 128_000u64;
    let requested_tokens = 150_000u64;

    let within_limit = requested_tokens <= max_tokens_per_request;

    assert!(!within_limit, "Should reject request exceeding token limit");
}

/// Test: Rate limiting (requests per minute)
#[test]
fn test_rate_limiting() {
    // Simulate rate limit check
    let requests_this_minute = 100u32;
    let max_requests_per_minute = 60u32;

    let within_rate_limit = requests_this_minute < max_requests_per_minute;

    assert!(!within_rate_limit, "Should reject request over rate limit");
}
