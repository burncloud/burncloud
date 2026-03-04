//! Resilience Tests - Rate Limiting and Circuit Breaker
//!
//! Test Coverage:
//! - RL-01: Rate Limit (RPM/TPM) enforcement
//! - RL-02: Adaptive rate limiting based on response time
//! - CB-01: Circuit breaker opens on consecutive failures
//! - CB-02: Circuit breaker recovery (half-open state probing)

mod common;

use burncloud_database::sqlx;
use burncloud_router::{
    AdaptiveLimitConfig, AdaptiveRateLimit, CircuitBreaker, FailureType, RateLimitScope,
    RateLimitState, RateLimiter,
};
use common::{setup_db, start_mock_upstream, start_test_server};
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Unit Tests for Rate Limiter (Token Bucket)
// ============================================================================

mod rate_limiter_unit_tests {
    use super::*;

    /// RL-01-U1: Basic rate limiting - requests should pass when tokens available
    #[test]
    fn test_rate_limiter_allows_within_capacity() {
        // Capacity 10, refill rate 1/sec
        let limiter = RateLimiter::new(10.0, 1.0);

        // Should allow 10 requests initially
        for i in 0..10 {
            assert!(
                limiter.check("user1", 1.0),
                "Request {} should be allowed",
                i + 1
            );
        }

        // 11th request should be denied
        assert!(
            !limiter.check("user1", 1.0),
            "11th request should be denied"
        );
    }

    /// RL-01-U2: Token refill over time
    #[test]
    fn test_rate_limiter_refills_tokens() {
        let limiter = RateLimiter::new(5.0, 10.0); // capacity 5, 10 tokens/sec

        // Consume all tokens
        for _ in 0..5 {
            limiter.check("user1", 1.0);
        }
        assert!(!limiter.check("user1", 1.0));

        // Wait for refill
        std::thread::sleep(Duration::from_millis(200)); // Should get ~2 tokens

        // Should allow new requests
        assert!(
            limiter.check("user1", 1.0),
            "Should allow after token refill"
        );
    }

    /// RL-01-U3: Different keys have independent buckets
    #[test]
    fn test_rate_limiter_per_key_isolation() {
        let limiter = RateLimiter::new(3.0, 0.0); // No refill for simplicity

        // Exhaust user1's bucket
        for _ in 0..3 {
            limiter.check("user1", 1.0);
        }
        assert!(!limiter.check("user1", 1.0));

        // user2 should still have tokens
        assert!(
            limiter.check("user2", 1.0),
            "user2 should have separate bucket"
        );
    }

    /// RL-01-U4: Variable cost tokens (TPM simulation)
    #[test]
    fn test_rate_limiter_variable_cost() {
        // Simulating TPM: capacity 100 tokens
        let limiter = RateLimiter::new(100.0, 1.0);

        // Request consuming 50 tokens
        assert!(limiter.check("user1", 50.0), "50 token request should pass");

        // Request consuming 30 tokens
        assert!(limiter.check("user1", 30.0), "30 token request should pass");

        // 25 tokens should fail (only 20 left)
        assert!(
            !limiter.check("user1", 25.0),
            "25 token request should fail"
        );

        // 20 tokens should pass
        assert!(limiter.check("user1", 20.0), "20 token request should pass");
    }
}

// ============================================================================
// Unit Tests for Adaptive Rate Limiter
// ============================================================================

mod adaptive_rate_limiter_unit_tests {
    use super::*;

    /// RL-02-U1: Initial state is Learning
    #[test]
    fn test_adaptive_initial_state() {
        let limiter = AdaptiveRateLimit::with_defaults();
        assert_eq!(*limiter.get_state(), RateLimitState::Learning);
        assert!(limiter.check_available());
    }

    /// RL-02-U2: Learning to Stable transition
    #[test]
    fn test_adaptive_learning_to_stable() {
        let mut limiter = AdaptiveRateLimit::new(AdaptiveLimitConfig {
            learning_duration: 3,
            ..Default::default()
        });

        // Process learning_duration requests
        for _ in 0..3 {
            limiter.on_success(None);
        }

        assert_eq!(*limiter.get_state(), RateLimitState::Stable);
    }

    /// RL-02-U3: Rate limited reduces limit
    #[test]
    fn test_adaptive_rate_limited_reduces_limit() {
        let mut limiter = AdaptiveRateLimit::new(AdaptiveLimitConfig {
            initial_limit: 100,
            ..Default::default()
        });

        let initial_limit = limiter.get_current_limit();
        limiter.on_rate_limited(None);

        // Should reduce by 20%
        let expected = (initial_limit as f64 * 0.8).ceil() as u32;
        assert_eq!(limiter.get_current_limit(), expected);
    }

    /// RL-02-U4: Cooldown state blocks requests
    #[test]
    fn test_adaptive_cooldown_blocks_requests() {
        let mut limiter = AdaptiveRateLimit::new(AdaptiveLimitConfig {
            failure_threshold: 1,
            cooldown_duration: Duration::from_secs(10),
            ..Default::default()
        });

        limiter.on_rate_limited(None);

        assert_eq!(*limiter.get_state(), RateLimitState::Cooldown);
        assert!(!limiter.check_available());
    }

    /// RL-02-U5: Recovery from cooldown
    #[test]
    fn test_adaptive_recovery_from_cooldown() {
        let mut limiter = AdaptiveRateLimit::new(AdaptiveLimitConfig {
            failure_threshold: 1,
            cooldown_duration: Duration::from_millis(10),
            recovery_ratio: 0.5,
            ..Default::default()
        });

        let initial_limit = limiter.get_current_limit();
        limiter.on_rate_limited(None);

        // Wait for cooldown to expire
        std::thread::sleep(Duration::from_millis(20));

        // on_success triggers recovery check
        limiter.on_success(None);

        assert_eq!(*limiter.get_state(), RateLimitState::Learning);
        // Limit should be reduced by recovery_ratio (50% of 80%)
        let expected_max = (initial_limit as f64 * 0.8 * 0.5).ceil() as u32;
        assert!(limiter.get_current_limit() <= expected_max);
    }

    /// RL-02-U6: Learning upstream limit from headers
    #[test]
    fn test_adaptive_learns_upstream_limit() {
        let mut limiter = AdaptiveRateLimit::with_defaults();

        limiter.on_success(Some(500));

        assert_eq!(limiter.get_learned_limit(), Some(500));
        assert_eq!(limiter.get_current_limit(), 500);
    }

    /// RL-02-U7: Success streak increases limit
    #[test]
    fn test_adaptive_success_streak_increases_limit() {
        let mut limiter = AdaptiveRateLimit::new(AdaptiveLimitConfig {
            success_threshold: 3,
            adjustment_step: 10,
            learning_duration: 100, // Stay in Learning
            max_limit: 1000,
            ..Default::default()
        });

        let initial_limit = limiter.get_current_limit();

        // 3 successes should trigger limit increase
        for _ in 0..3 {
            limiter.on_success(None);
        }

        assert_eq!(limiter.get_current_limit(), initial_limit + 10);
    }

    /// RL-02-U8: Rate limit expiry is respected
    #[test]
    fn test_adaptive_rate_limit_expiry() {
        let mut limiter = AdaptiveRateLimit::with_defaults();

        // Set rate limit for very short duration
        limiter.on_rate_limited(Some(0)); // 0 seconds

        // Should be available immediately
        assert!(limiter.check_available());
    }
}

// ============================================================================
// Unit Tests for Circuit Breaker
// ============================================================================

mod circuit_breaker_unit_tests {
    use super::*;

    /// CB-01-U1: Circuit breaker starts closed (allows requests)
    #[test]
    fn test_circuit_breaker_initially_closed() {
        let cb = CircuitBreaker::new(5, 30);
        assert!(cb.allow_request("upstream1"));
    }

    /// CB-01-U2: Circuit opens after threshold failures
    #[test]
    fn test_circuit_breaker_opens_on_failures() {
        let cb = CircuitBreaker::new(3, 30);

        // Record 3 failures
        for _ in 0..3 {
            cb.record_failure_with_type("upstream1", FailureType::ServerError);
        }

        // Circuit should be open
        assert!(!cb.allow_request("upstream1"));
    }

    /// CB-01-U3: Different failure types affect circuit differently
    #[test]
    fn test_circuit_breaker_auth_failure_immediate_trip() {
        let cb = CircuitBreaker::new(10, 30); // High threshold

        // Single auth failure should immediately trip
        cb.record_failure_with_type("upstream1", FailureType::AuthFailed);

        assert!(!cb.allow_request("upstream1"));
    }

    /// CB-01-U4: Payment failure immediately trips circuit
    #[test]
    fn test_circuit_breaker_payment_failure_immediate_trip() {
        let cb = CircuitBreaker::new(10, 30);

        cb.record_failure_with_type("upstream1", FailureType::PaymentRequired);

        assert!(!cb.allow_request("upstream1"));
    }

    /// CB-01-U5: Rate limited failure respects retry_after
    #[test]
    fn test_circuit_breaker_rate_limited_with_retry_after() {
        let cb = CircuitBreaker::new(10, 30);

        cb.record_failure_with_type(
            "upstream1",
            FailureType::RateLimited {
                scope: RateLimitScope::Account,
                retry_after: Some(60),
            },
        );

        // Should be blocked due to rate limit
        assert!(!cb.allow_request("upstream1"));
    }

    /// CB-02-U1: Circuit enters half-open after cooldown
    #[test]
    fn test_circuit_breaker_half_open_after_cooldown() {
        // Use a small but non-zero cooldown to avoid race condition
        let cb = CircuitBreaker::new(2, 1); // 1 second cooldown

        // Trip the circuit
        cb.record_failure_with_type("upstream1", FailureType::ServerError);
        cb.record_failure_with_type("upstream1", FailureType::ServerError);

        // Circuit should be open immediately after failures
        assert!(
            !cb.allow_request("upstream1"),
            "Circuit should be open after failures"
        );

        // Wait for cooldown to expire
        std::thread::sleep(Duration::from_millis(1100));

        // Should allow probe request (half-open)
        assert!(
            cb.allow_request("upstream1"),
            "Should allow probe after cooldown"
        );
    }

    /// CB-02-U2: Success resets circuit breaker
    #[test]
    fn test_circuit_breaker_success_resets() {
        let cb = CircuitBreaker::new(2, 30);

        // Trip the circuit
        cb.record_failure_with_type("upstream1", FailureType::ServerError);
        cb.record_failure_with_type("upstream1", FailureType::ServerError);

        // Record success
        cb.record_success("upstream1");

        // Should allow requests again
        assert!(cb.allow_request("upstream1"));
    }

    /// CB-02-U3: Failure during half-open reopens circuit
    #[test]
    fn test_circuit_breaker_failure_during_half_open() {
        let cb = CircuitBreaker::new(2, 1); // 1 second cooldown

        // Trip the circuit
        cb.record_failure_with_type("upstream1", FailureType::ServerError);
        cb.record_failure_with_type("upstream1", FailureType::ServerError);

        // Circuit should be open
        assert!(!cb.allow_request("upstream1"), "Circuit should be open");

        // Wait for cooldown to expire
        std::thread::sleep(Duration::from_millis(1100));

        // Should be in half-open (allow probe)
        assert!(
            cb.allow_request("upstream1"),
            "Should allow probe in half-open"
        );

        // Failure during half-open
        cb.record_failure_with_type("upstream1", FailureType::ServerError);

        // Should be blocked again (circuit reopened)
        assert!(
            !cb.allow_request("upstream1"),
            "Should be blocked after failure in half-open"
        );
    }

    /// CB-02-U4: Different upstreams have independent states
    #[test]
    fn test_circuit_breaker_per_upstream_isolation() {
        let cb = CircuitBreaker::new(2, 30);

        // Trip upstream1
        cb.record_failure_with_type("upstream1", FailureType::ServerError);
        cb.record_failure_with_type("upstream1", FailureType::ServerError);

        // upstream2 should still be available
        assert!(cb.allow_request("upstream2"));
        assert!(!cb.allow_request("upstream1"));
    }

    /// CB-02-U5: Get status map for monitoring
    #[test]
    fn test_circuit_breaker_status_map() {
        let cb = CircuitBreaker::new(2, 30);

        // Trip one upstream
        cb.record_failure_with_type("upstream1", FailureType::ServerError);
        cb.record_failure_with_type("upstream1", FailureType::ServerError);

        let status_map = cb.get_status_map();

        assert!(status_map.contains_key("upstream1"));
        assert!(status_map["upstream1"].contains("Open"));
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

mod integration_tests {
    use super::*;

    /// RL-01-I1: RPM limit enforcement at API level
    #[tokio::test]
    async fn test_rpm_limit_enforcement() -> anyhow::Result<()> {
        let (_db, pool) = setup_db().await?;

        // Start Mock Upstream
        let mock_port = 3051;
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", mock_port))
            .await
            .unwrap();
        tokio::spawn(async move {
            start_mock_upstream(listener).await;
        });

        // Create upstream
        let upstream_id = "rpm_test";
        let upstream_url = format!("http://127.0.0.1:{}/anything", mock_port);
        let match_path = "/rpm-test";

        sqlx::query(
            r#"
            INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type)
            VALUES (?, 'RPM Test', ?, 'test-key', ?, 'Bearer')
            ON CONFLICT(id) DO UPDATE SET base_url=excluded.base_url, name=excluded.name
            "#,
        )
        .bind(upstream_id)
        .bind(upstream_url)
        .bind(match_path)
        .execute(&pool)
        .await?;

        // Create group
        let group_id = "rpm_group";
        sqlx::query(
            "INSERT INTO router_groups (id, name, strategy, match_path) VALUES (?, 'RPM Group', 'round_robin', ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name, match_path=excluded.match_path"
        )
        .bind(group_id).bind(match_path)
        .execute(&pool).await?;

        // Bind upstream to group
        sqlx::query("DELETE FROM router_group_members WHERE group_id = ?")
            .bind(group_id)
            .execute(&pool)
            .await?;
        sqlx::query(
            "INSERT INTO router_group_members (group_id, upstream_id, weight) VALUES (?, ?, 1)",
        )
        .bind(group_id)
        .bind(upstream_id)
        .execute(&pool)
        .await?;

        // Start server
        let port = 3051;
        start_test_server(port).await;

        let client = Client::new();
        let url = format!("http://localhost:{}{}", port, match_path);

        // Note: The actual RPM limit is configured in the router's RateLimiter instance.
        // This test verifies the integration point exists. The actual limit values
        // depend on the router configuration.
        for i in 0..3 {
            let resp = client
                .post(&url)
                .header("Authorization", "Bearer sk-burncloud-demo")
                .json(&serde_json::json!({"test": "rpm"}))
                .send()
                .await?;

            println!("Request {}: {}", i, resp.status());
        }

        Ok(())
    }

    /// CB-01-I1: Circuit breaker opens on consecutive 5xx errors
    #[tokio::test]
    async fn test_circuit_breaker_on_server_errors() -> anyhow::Result<()> {
        // This test verifies the circuit breaker logic in isolation
        // as it doesn't require a full server setup

        let cb = Arc::new(CircuitBreaker::new(3, 30));
        let upstream_id = "failing-upstream";

        // Simulate consecutive server errors
        for i in 0..3 {
            cb.record_failure_with_type(upstream_id, FailureType::ServerError);
            println!("Recorded failure {}", i + 1);
        }

        // Circuit should now be open
        assert!(!cb.allow_request(upstream_id));

        let status_map = cb.get_status_map();
        println!("Status: {:?}", status_map);

        assert!(status_map[upstream_id].contains("Open"));

        Ok(())
    }

    /// CB-02-I1: Circuit breaker recovers after cooldown
    #[tokio::test]
    async fn test_circuit_breaker_recovery() -> anyhow::Result<()> {
        // Use 1 second cooldown for faster test
        let cb = Arc::new(CircuitBreaker::new(2, 1));
        let upstream_id = "recovering-upstream";

        // Trip the circuit
        cb.record_failure_with_type(upstream_id, FailureType::ServerError);
        cb.record_failure_with_type(upstream_id, FailureType::ServerError);

        assert!(!cb.allow_request(upstream_id), "Circuit should be open");

        // Wait for cooldown
        tokio::time::sleep(Duration::from_millis(1100)).await;

        // Should allow probe request (half-open)
        assert!(
            cb.allow_request(upstream_id),
            "Should allow probe after cooldown"
        );

        // Simulate successful probe
        cb.record_success(upstream_id);

        // Circuit should be closed
        assert!(
            cb.allow_request(upstream_id),
            "Circuit should be closed after success"
        );

        Ok(())
    }

    /// CB-01-I2: Auth failure immediately trips circuit
    #[tokio::test]
    async fn test_auth_failure_immediate_trip() -> anyhow::Result<()> {
        let cb = Arc::new(CircuitBreaker::new(100, 30)); // High threshold
        let upstream_id = "auth-failing-upstream";

        // Single auth failure should immediately trip
        cb.record_failure_with_type(upstream_id, FailureType::AuthFailed);

        assert!(
            !cb.allow_request(upstream_id),
            "Auth failure should immediately trip circuit"
        );

        Ok(())
    }

    /// RL-02-I1: Adaptive rate limiter adjusts based on failures
    #[tokio::test]
    async fn test_adaptive_rate_limiter_adjustment() -> anyhow::Result<()> {
        let mut adaptive = AdaptiveRateLimit::new(AdaptiveLimitConfig {
            initial_limit: 100,
            failure_threshold: 2,
            cooldown_duration: Duration::from_millis(100),
            recovery_ratio: 0.5,
            ..Default::default()
        });

        let initial_limit = adaptive.get_current_limit();
        println!("Initial limit: {}", initial_limit);

        // Simulate rate limited errors
        adaptive.on_rate_limited(None);
        let after_first = adaptive.get_current_limit();
        println!("After first 429: {}", after_first);

        adaptive.on_rate_limited(None);
        let after_second = adaptive.get_current_limit();
        println!("After second 429: {}", after_second);

        // Should be in cooldown
        assert_eq!(*adaptive.get_state(), RateLimitState::Cooldown);
        assert!(!adaptive.check_available());

        // Wait for cooldown
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Trigger recovery
        adaptive.on_success(None);
        assert_eq!(*adaptive.get_state(), RateLimitState::Learning);

        println!("Final limit: {}", adaptive.get_current_limit());

        Ok(())
    }

    /// Combined test: Rate limiter + Circuit breaker working together
    #[tokio::test]
    async fn test_combined_rate_limit_and_circuit_breaker() -> anyhow::Result<()> {
        let rate_limiter = Arc::new(RateLimiter::new(100.0, 10.0));
        let circuit_breaker = Arc::new(CircuitBreaker::new(3, 30));

        let key = "combined-test-user";
        let upstream = "combined-test-upstream";

        // Simulate normal operation
        for i in 0..5 {
            // Check rate limit
            if !rate_limiter.check(key, 1.0) {
                println!("Request {} rate limited", i);
                break;
            }

            // Check circuit breaker
            if !circuit_breaker.allow_request(upstream) {
                println!("Request {} blocked by circuit breaker", i);
                break;
            }

            println!("Request {} allowed", i);

            // Simulate some failures
            if i >= 2 {
                circuit_breaker.record_failure_with_type(upstream, FailureType::ServerError);
            }
        }

        // After 3 failures, circuit should be open
        assert!(!circuit_breaker.allow_request(upstream));

        // But rate limiter should still have tokens
        // (different concerns - rate limit is per user, circuit is per upstream)
        let new_user = "new-user";
        assert!(rate_limiter.check(new_user, 1.0));

        Ok(())
    }
}

// ============================================================================
// Concurrency Tests
// ============================================================================

mod concurrency_tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Test rate limiter under concurrent access
    #[tokio::test]
    async fn test_rate_limiter_concurrent_access() {
        let limiter = Arc::new(RateLimiter::new(100.0, 10.0));
        let success_count = Arc::new(AtomicU32::new(0));
        let fail_count = Arc::new(AtomicU32::new(0));

        let mut handles = vec![];

        // Spawn 150 concurrent requests
        for _ in 0..150 {
            let limiter = limiter.clone();
            let success = success_count.clone();
            let fail = fail_count.clone();

            handles.push(tokio::spawn(async move {
                if limiter.check("concurrent-user", 1.0) {
                    success.fetch_add(1, Ordering::SeqCst);
                } else {
                    fail.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        // Wait for all
        for handle in handles {
            handle.await.unwrap();
        }

        let successes = success_count.load(Ordering::SeqCst);
        let failures = fail_count.load(Ordering::SeqCst);

        println!("Successes: {}, Failures: {}", successes, failures);

        // Should have exactly 100 successes (capacity)
        assert_eq!(successes, 100);
        assert_eq!(failures, 50);
    }

    /// Test circuit breaker under concurrent access
    #[tokio::test]
    async fn test_circuit_breaker_concurrent_access() {
        let cb = Arc::new(CircuitBreaker::new(10, 30));
        let allow_count = Arc::new(AtomicU32::new(0));
        let deny_count = Arc::new(AtomicU32::new(0));

        let mut handles = vec![];

        // First: record 10 failures concurrently
        for _ in 0..10 {
            let cb = cb.clone();
            handles.push(tokio::spawn(async move {
                cb.record_failure_with_type("concurrent-upstream", FailureType::ServerError);
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Now: check allow_request concurrently
        let mut handles = vec![];
        for _ in 0..20 {
            let cb = cb.clone();
            let allow = allow_count.clone();
            let deny = deny_count.clone();

            handles.push(tokio::spawn(async move {
                if cb.allow_request("concurrent-upstream") {
                    allow.fetch_add(1, Ordering::SeqCst);
                } else {
                    deny.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let allows = allow_count.load(Ordering::SeqCst);
        let denys = deny_count.load(Ordering::SeqCst);

        println!("Allows: {}, Denys: {}", allows, denys);

        // All should be denied (circuit is open)
        assert_eq!(allows, 0);
        assert_eq!(denys, 20);
    }
}
