use burncloud_node_runtime::{FakeHealthProbe, HealthProbe, ReadinessTarget};

/// Readiness answers whether a process may become routable; health is a
/// continuing observation used later for removal/recovery. Keeping separate
/// ports prevents a successful spawn or one-time readiness event from being
/// treated as permanent health.
#[tokio::test]
async fn health_is_an_explicit_independent_observation() {
    let target = ReadinessTarget {
        endpoint: "fake://health".into(),
    };
    assert!(FakeHealthProbe.is_healthy(target).await.unwrap());
}
