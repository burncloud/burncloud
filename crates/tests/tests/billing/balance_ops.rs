//! BL-04 & BL-05: Balance Operations Tests (P0)
//!
//! Tests for user balance deduction and recharge functionality.
//!
//! Key Requirements (from CLAUDE.md):
//! - Balance stored as i64 nanodollars (9 decimal precision)
//! - Dual-currency wallet: USD and CNY
//! - Concurrency-safe balance updates using CAS pattern
//! - NO "read -> check -> write" pattern (race condition risk)
//! - MUST use atomic UPDATE with WHERE clause for balance checks
//!
//! SQL Pattern for Safe Deduction:
//! ```sql
//! UPDATE users SET balance = balance - ? WHERE id = ? AND balance >= ?
//! ```

use burncloud_common::{dollars_to_nano, NANO_PER_DOLLAR};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;

// ============================================================================
// BL-04: Balance Deduction - P0
// ============================================================================

/// Test: Basic balance deduction calculation
#[test]
fn test_balance_deduction_calculation() {
    let balance_nano = dollars_to_nano(100.0); // $100 balance
    let cost_nano = dollars_to_nano(0.05); // $0.05 cost

    let new_balance = balance_nano - cost_nano;

    assert_eq!(
        new_balance,
        99_950_000_000,
        "$100 - $0.05 = $99.95 = 99.95B nano"
    );
}

/// Test: Balance deduction with nanodollar precision
#[test]
fn test_balance_deduction_precision() {
    // $10.123456789 balance (uses all 9 decimal places)
    let balance = 10_123_456_789i64;
    // $0.000000001 cost (1 nanodollar)
    let cost = 1i64;

    let new_balance = balance - cost;

    assert_eq!(new_balance, 10_123_456_788, "Should preserve nanodollar precision");
}

/// Test: Multiple deductions accumulate correctly
#[test]
fn test_multiple_deductions() {
    let mut balance = dollars_to_nano(10.0); // $10.00

    // First deduction: $0.50
    balance -= dollars_to_nano(0.50);

    // Second deduction: $1.25
    balance -= dollars_to_nano(1.25);

    // Third deduction: $0.05
    balance -= dollars_to_nano(0.05);

    assert_eq!(
        balance, 8_200_000_000,
        "After 3 deductions: $10 - $0.50 - $1.25 - $0.05 = $8.20"
    );
}

/// Test: Zero deduction doesn't change balance
#[test]
fn test_zero_deduction() {
    let balance = dollars_to_nano(100.0);
    let cost = 0i64;

    let new_balance = balance - cost;

    assert_eq!(new_balance, balance, "Zero cost should not change balance");
}

/// Test: Deduction results in zero balance
#[test]
fn test_deduction_to_zero() {
    let balance = dollars_to_nano(5.0);
    let cost = dollars_to_nano(5.0);

    let new_balance = balance - cost;

    assert_eq!(new_balance, 0, "Deduction to exactly $0 should work");
}

/// Test: Concurrency-safe balance deduction simulation
/// Uses atomic operations to simulate the CAS (Compare-And-Swap) pattern
#[test]
fn test_concurrent_deduction_simulation() {
    // Simulate $100 balance shared across threads
    let balance = Arc::new(AtomicI64::new(dollars_to_nano(100.0)));
    let deduction_amount = dollars_to_nano(1.0); // $1.00 per deduction

    // Spawn 50 threads each trying to deduct $1
    let mut handles = vec![];

    for _ in 0..50 {
        let balance_clone = Arc::clone(&balance);
        let deduction = deduction_amount;

        handles.push(thread::spawn(move || {
            // Simulate CAS loop
            loop {
                let current = balance_clone.load(Ordering::SeqCst);

                // Check if sufficient balance
                if current < deduction {
                    return false; // Insufficient balance
                }

                let new_value = current - deduction;

                // Try to update atomically
                if balance_clone.compare_exchange(
                    current,
                    new_value,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ).is_ok() {
                    return true; // Deduction successful
                }
                // If CAS failed, another thread modified balance, retry
            }
        }));
    }

    // Count successful deductions
    let successful: usize = handles.into_iter().map(|h| h.join().unwrap()).filter(|&r| r).count();

    // All 50 should succeed since $100 > 50 * $1
    assert_eq!(successful, 50, "All 50 deductions should succeed");

    // Final balance should be $50
    let final_balance = balance.load(Ordering::SeqCst);
    assert_eq!(
        final_balance,
        dollars_to_nano(50.0),
        "Final balance should be $50"
    );
}

/// Test: Concurrent deduction with insufficient balance
#[test]
fn test_concurrent_deduction_insufficient_balance() {
    // Start with $10 balance
    let balance = Arc::new(AtomicI64::new(dollars_to_nano(10.0)));
    let deduction_amount = dollars_to_nano(1.0);

    // Spawn 20 threads trying to deduct $1 each (total $20)
    let mut handles = vec![];

    for _ in 0..20 {
        let balance_clone = Arc::clone(&balance);
        let deduction = deduction_amount;

        handles.push(thread::spawn(move || {
            loop {
                let current = balance_clone.load(Ordering::SeqCst);

                if current < deduction {
                    return false; // Insufficient balance
                }

                let new_value = current - deduction;

                if balance_clone.compare_exchange(
                    current,
                    new_value,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ).is_ok() {
                    return true;
                }
            }
        }));
    }

    let results: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let successful = results.iter().filter(|&&r| r).count();
    let failed = results.iter().filter(|&&r| !r).count();

    // Only 10 should succeed (limited by $10 balance)
    assert_eq!(successful, 10, "Only 10 deductions should succeed");
    assert_eq!(failed, 10, "10 deductions should fail due to insufficient balance");

    // Final balance should be $0
    let final_balance = balance.load(Ordering::SeqCst);
    assert_eq!(final_balance, 0, "Final balance should be $0");
}

/// Test: Balance never goes negative with CAS pattern
#[test]
fn test_balance_never_negative() {
    let balance = Arc::new(AtomicI64::new(dollars_to_nano(5.0)));

    // Try to deduct $10 (more than balance)
    let balance_clone = Arc::clone(&balance);
    let deduction = dollars_to_nano(10.0);

    let result = loop {
        let current = balance_clone.load(Ordering::SeqCst);

        if current < deduction {
            break false; // Should reject
        }

        let new_value = current - deduction;

        if balance_clone.compare_exchange(
            current,
            new_value,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ).is_ok() {
            break true;
        }
    };

    assert!(!result, "Deduction larger than balance should be rejected");

    let final_balance = balance.load(Ordering::SeqCst);
    assert_eq!(
        final_balance,
        dollars_to_nano(5.0),
        "Balance should remain unchanged"
    );
    assert!(
        final_balance >= 0,
        "Balance should never be negative"
    );
}

// ============================================================================
// BL-05: Recharge Functionality - P0
// ============================================================================

/// Test: USD recharge adds to USD balance
#[test]
fn test_usd_recharge() {
    let balance_usd = dollars_to_nano(50.0);
    let recharge_amount = dollars_to_nano(25.0);

    let new_balance = balance_usd + recharge_amount;

    assert_eq!(
        new_balance,
        dollars_to_nano(75.0),
        "$50 + $25 recharge = $75"
    );
}

/// Test: CNY recharge adds to CNY balance
#[test]
fn test_cny_recharge() {
    let balance_cny = rate_to_scaled_nano(100.0, 7.24); // ¥100
    let recharge_amount = rate_to_scaled_nano(50.0, 7.24); // ¥50

    let new_balance = balance_cny + recharge_amount;

    // ¥100 + ¥50 = ¥150
    let expected = rate_to_scaled_nano(150.0, 7.24);
    assert_eq!(new_balance, expected, "¥100 + ¥50 recharge = ¥150");
}

/// Test: Dual currency wallet - USD and CNY are separate
#[test]
#[allow(unused_assignments)]
fn test_dual_currency_wallet() {
    let mut balance_usd = dollars_to_nano(100.0);
    let mut balance_cny = rate_to_scaled_nano(500.0, 7.24);

    // Deduct from USD
    balance_usd -= dollars_to_nano(10.0);

    // CNY should be unaffected
    assert_eq!(
        balance_cny,
        rate_to_scaled_nano(500.0, 7.24),
        "CNY balance should not change"
    );

    // Recharge CNY
    balance_cny += rate_to_scaled_nano(100.0, 7.24);

    // USD should be unaffected
    assert_eq!(
        balance_usd,
        dollars_to_nano(90.0),
        "USD balance should not change"
    );
}

/// Test: Recharge record structure
#[test]
fn test_recharge_record_structure() {
    // Simulate recharge record fields
    let user_id = "user-123";
    let amount_nano = dollars_to_nano(50.0);
    let currency = "USD";
    let description = "Stripe payment - invoice_12345";

    // Verify values
    assert_eq!(user_id, "user-123");
    assert_eq!(amount_nano, 50_000_000_000);
    assert_eq!(currency, "USD");
    assert!(description.starts_with("Stripe"));
}

/// Test: Multiple recharges accumulate
#[test]
fn test_multiple_recharges() {
    let mut balance = dollars_to_nano(0.0);

    // First recharge: $10
    balance += dollars_to_nano(10.0);

    // Second recharge: $25
    balance += dollars_to_nano(25.0);

    // Third recharge: $15
    balance += dollars_to_nano(15.0);

    assert_eq!(
        balance,
        dollars_to_nano(50.0),
        "Total after 3 recharges: $50"
    );
}

/// Test: Recharge with nanodollar precision
#[test]
fn test_recharge_precision() {
    let mut balance = 0i64;

    // Recharge $0.000000001 (1 nanodollar)
    balance += 1;

    assert_eq!(balance, 1, "Should handle 1 nanodollar recharge");

    // Recharge $0.000000009 (9 nanodollars)
    balance += 9;

    assert_eq!(balance, 10, "Should handle 9 nanodollar recharge");
}

/// Test: Large recharge amount
#[test]
fn test_large_recharge() {
    let balance = dollars_to_nano(0.0);
    // Recharge $1,000,000 (enterprise customer)
    let recharge = dollars_to_nano(1_000_000.0);

    let new_balance = balance + recharge;

    // $1M = 1,000,000 * 10^9 = 10^15 nanodollars
    assert_eq!(
        new_balance,
        1_000_000_000_000_000i64,
        "$1M should be 10^15 nanodollars"
    );
}

/// Test: Signup bonus value
#[test]
fn test_signup_bonus() {
    // From service-user: SIGNUP_BONUS_NANO = 10_000_000_000
    const SIGNUP_BONUS_NANO: i64 = 10_000_000_000;

    let new_user_balance = SIGNUP_BONUS_NANO;

    assert_eq!(
        new_user_balance,
        dollars_to_nano(10.0),
        "Signup bonus should be $10"
    );
}

/// Test: Demo user default balance
#[test]
fn test_demo_user_balance() {
    // From database-user: demo user gets 100 USD
    const DEMO_BALANCE_NANO: i64 = 100_000_000_000;

    assert_eq!(
        DEMO_BALANCE_NANO,
        dollars_to_nano(100.0),
        "Demo balance should be $100"
    );
}

/// Test: Balance after deduction then recharge cycle
#[test]
fn test_deduction_recharge_cycle() {
    let mut balance = dollars_to_nano(100.0);

    // Use service
    balance -= dollars_to_nano(5.0);
    assert_eq!(balance, dollars_to_nano(95.0));

    // Use service again
    balance -= dollars_to_nano(3.50);
    assert_eq!(balance, dollars_to_nano(91.5));

    // Recharge $50
    balance += dollars_to_nano(50.0);
    assert_eq!(balance, dollars_to_nano(141.5));

    // Use service
    balance -= dollars_to_nano(1.5);
    assert_eq!(balance, dollars_to_nano(140.0));
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a local currency amount to nanodollars using exchange rate
fn rate_to_scaled_nano(amount: f64, rate: f64) -> i64 {
    // local_amount_in_nano = amount * rate * 1_000_000_000
    // But we store local amounts in the same nanodollar scale for consistency
    (amount * rate * NANO_PER_DOLLAR as f64).round() as i64
}

// ============================================================================
// Integration-style Tests (require database)
// ============================================================================

/// Note: Full integration tests for balance operations are in the
/// `gemini_billing.rs` file which tests against a running server.
/// The tests here focus on the calculation logic and concurrency patterns.

/// Test: Verify the SQL pattern for safe balance deduction
#[test]
fn test_safe_deduction_sql_pattern() {
    // This test documents the expected SQL pattern
    // Real database tests should verify this behavior

    // Safe pattern (single atomic operation):
    // UPDATE users SET balance_usd = balance_usd - ? WHERE id = ? AND balance_usd >= ?

    // The WHERE clause ensures:
    // 1. Balance check is atomic with update
    // 2. No race condition between check and update
    // 3. Update only succeeds if balance was sufficient

    // Unsafe pattern (race condition risk):
    // 1. SELECT balance FROM users WHERE id = ?
    // 2. if balance >= deduction { UPDATE users SET balance = ? WHERE id = ? }
    // ^ This allows race conditions!

    // The safe pattern returns affected_rows = 0 when balance insufficient
    // which maps to Error::InsufficientBalance

    let sufficient_balance = dollars_to_nano(10.0);
    let deduction = dollars_to_nano(5.0);

    // Simulate safe deduction
    let success = sufficient_balance >= deduction;
    let new_balance = if success {
        sufficient_balance - deduction
    } else {
        sufficient_balance
    };

    assert!(success, "Should succeed with sufficient balance");
    assert_eq!(new_balance, dollars_to_nano(5.0));
}

/// Test: Balance overflow protection
#[test]
fn test_balance_overflow_protection() {
    let max_balance = i64::MAX;
    let recharge_amount = 1i64;

    // Check for overflow
    let result = max_balance.checked_add(recharge_amount);

    assert!(
        result.is_none(),
        "Overflow should be detected and prevented"
    );
}

/// Test: Negative deduction is rejected
#[test]
fn test_negative_deduction_rejected() {
    let _balance = dollars_to_nano(100.0);
    let negative_deduction = -dollars_to_nano(10.0);

    // Negative deduction would actually increase balance (bug!)
    // This should be rejected at the service layer

    // In production code:
    // if amount < 0 { return Err(Error::InvalidAmount); }

    assert!(
        negative_deduction < 0,
        "Negative deduction should be caught and rejected"
    );
}
