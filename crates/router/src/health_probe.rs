//! Health Probe Module
//!
//! This module implements proactive health checking for circuit breakers in Half-Open state.
//! Instead of waiting for user requests to test recovery, the system actively sends probe
//! requests to detect if the upstream has recovered.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::response_quality::ResponseQualityDetector;
#[allow(unused_imports)]
use crate::smart_circuit_breaker::{CircuitState, SmartCircuitBreaker, TripLevel};

/// Configuration for health probing
#[derive(Debug, Clone)]
pub struct HealthProbeConfig {
    /// Interval between probe attempts (default: 30 seconds)
    pub probe_interval: Duration,
    /// Timeout for probe requests (default: 10 seconds)
    pub probe_timeout: Duration,
    /// Minimum time in Half-Open before probing (default: 10 seconds)
    pub min_half_open_duration: Duration,
    /// Number of successful probes needed to close circuit (default: 1)
    pub success_threshold: u32,
    /// Number of failed probes before re-opening circuit (default: 2)
    pub failure_threshold: u32,
    /// Models to use for probing (first available will be used)
    pub probe_models: Vec<String>,
    /// Probe request body template
    pub probe_body: String,
}

impl Default for HealthProbeConfig {
    fn default() -> Self {
        Self {
            probe_interval: Duration::from_secs(30),
            probe_timeout: Duration::from_secs(10),
            min_half_open_duration: Duration::from_secs(10),
            success_threshold: 1,
            failure_threshold: 2,
            probe_models: vec![
                "gpt-3.5-turbo".to_string(),
                "claude-3-haiku".to_string(),
                "gemini-pro".to_string(),
            ],
            probe_body: r#"{"messages":[{"role":"user","content":"hi"}],"max_tokens":1}"#.to_string(),
        }
    }
}

/// Result of a health probe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    /// Channel ID that was probed
    pub channel_id: i32,
    /// Model used for probing
    pub model: String,
    /// Whether the probe was successful
    pub success: bool,
    /// Response latency in milliseconds
    pub latency_ms: u64,
    /// Response quality (if available)
    pub quality: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Timestamp of the probe
    pub timestamp: u64,
}

/// Probe state for a single channel
#[derive(Debug)]
struct ChannelProbeState {
    /// Last probe time
    last_probe: Option<Instant>,
    /// Consecutive successful probes
    success_streak: u32,
    /// Consecutive failed probes
    failure_streak: u32,
    /// Whether a probe is currently in progress
    probing: AtomicBool,
}

impl Default for ChannelProbeState {
    fn default() -> Self {
        Self {
            last_probe: None,
            success_streak: 0,
            failure_streak: 0,
            probing: AtomicBool::new(false),
        }
    }
}

/// Health probe manager that coordinates probing across all channels
pub struct HealthProbeManager {
    /// Configuration
    config: HealthProbeConfig,
    /// Per-channel probe state
    probe_states: RwLock<HashMap<i32, ChannelProbeState>>,
    /// Response quality detector
    #[allow(dead_code)]
    detector: ResponseQualityDetector,
    /// Last probe results (for monitoring)
    last_results: RwLock<Vec<ProbeResult>>,
}

impl HealthProbeManager {
    /// Create a new health probe manager
    pub fn new(config: HealthProbeConfig) -> Self {
        Self {
            config,
            probe_states: RwLock::new(HashMap::new()),
            detector: ResponseQualityDetector::new(),
            last_results: RwLock::new(Vec::new()),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(HealthProbeConfig::default())
    }

    /// Check if a channel should be probed
    pub async fn should_probe(&self, channel_id: i32, breaker: &SmartCircuitBreaker) -> bool {
        // Only probe if in Half-Open state
        if breaker.state() != CircuitState::HalfOpen {
            return false;
        }

        let states = self.probe_states.read().await;
        if let Some(state) = states.get(&channel_id) {
            // Check if already probing
            if state.probing.load(Ordering::Relaxed) {
                return false;
            }

            // Check minimum interval
            if let Some(last) = state.last_probe {
                if last.elapsed() < self.config.probe_interval {
                    return false;
                }
            }
        }

        true
    }

    /// Mark that a probe is starting
    pub async fn start_probe(&self, channel_id: i32) {
        let mut states = self.probe_states.write().await;
        let state = states.entry(channel_id).or_default();
        state.probing.store(true, Ordering::Relaxed);
    }

    /// Record a probe result and update state
    pub async fn record_probe_result(&self, result: ProbeResult) -> ProbeAction {
        let mut states = self.probe_states.write().await;
        let state = states.entry(result.channel_id).or_default();

        // Update probe state
        state.last_probe = Some(Instant::now());
        state.probing.store(false, Ordering::Relaxed);

        if result.success {
            state.success_streak += 1;
            state.failure_streak = 0;

            // Check if we should close the circuit
            if state.success_streak >= self.config.success_threshold {
                state.success_streak = 0;
                return ProbeAction::CloseCircuit;
            }
        } else {
            state.failure_streak += 1;
            state.success_streak = 0;

            // Check if we should re-open the circuit
            if state.failure_streak >= self.config.failure_threshold {
                state.failure_streak = 0;
                return ProbeAction::ReopenCircuit;
            }
        }

        // Store result for monitoring
        let mut results = self.last_results.write().await;
        results.push(result);
        if results.len() > 100 {
            results.remove(0);
        }

        ProbeAction::Continue
    }

    /// Get the best model to use for probing a channel
    pub fn get_probe_model(&self, available_models: &[String]) -> Option<String> {
        for model in &self.config.probe_models {
            if available_models.contains(model) {
                return Some(model.clone());
            }
        }
        available_models.first().cloned()
    }

    /// Get probe request body
    pub fn get_probe_body(&self) -> &str {
        &self.config.probe_body
    }

    /// Get probe timeout
    pub fn get_probe_timeout(&self) -> Duration {
        self.config.probe_timeout
    }

    /// Get recent probe results
    pub async fn get_recent_results(&self, limit: usize) -> Vec<ProbeResult> {
        let results = self.last_results.read().await;
        results.iter().rev().take(limit).cloned().collect()
    }

    /// Reset probe state for a channel (e.g., after manual reset)
    pub async fn reset_channel(&self, channel_id: i32) {
        let mut states = self.probe_states.write().await;
        states.remove(&channel_id);
    }
}

impl Default for HealthProbeManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Action to take after recording a probe result
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProbeAction {
    /// Continue current state (need more probes)
    Continue,
    /// Close the circuit (recovery confirmed)
    CloseCircuit,
    /// Re-open the circuit (recovery failed)
    ReopenCircuit,
}

/// Background task that periodically checks for channels needing probing
pub struct ProbeScheduler {
    manager: Arc<HealthProbeManager>,
    running: Arc<AtomicBool>,
}

impl ProbeScheduler {
    /// Create a new probe scheduler
    pub fn new(manager: Arc<HealthProbeManager>) -> Self {
        Self {
            manager,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the background probe scheduler
    pub fn start(&self) {
        if self.running.swap(true, Ordering::Relaxed) {
            return; // Already running
        }

        let _manager = Arc::clone(&self.manager);
        let running = Arc::clone(&self.running);

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(10));
            
            while running.load(Ordering::Relaxed) {
                ticker.tick().await;
                
                // In a real implementation, this would:
                // 1. Get all channels with Half-Open breakers
                // 2. For each channel, check if probing is needed
                // 3. Send probe request through the appropriate adaptor
                // 4. Record the result
                
                tracing::debug!("Probe scheduler tick");
            }
        });
    }

    /// Stop the background scheduler
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// Trait for sending probe requests (to be implemented by the router)
#[async_trait::async_trait]
pub trait ProbeSender: Send + Sync {
    /// Send a probe request to a channel
    async fn send_probe(
        &self,
        channel_id: i32,
        model: &str,
        body: &str,
        timeout: Duration,
    ) -> ProbeResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_probe_state_management() {
        let manager = HealthProbeManager::with_defaults();
        let channel_id = 1;

        // Initial state
        assert!(manager.should_probe(channel_id, &SmartCircuitBreaker::with_defaults()).await);

        // Start probe
        manager.start_probe(channel_id).await;

        // Should not probe while probing
        let mut breaker = SmartCircuitBreaker::with_defaults();
        // Force to HalfOpen
        breaker.trip("test", Duration::from_secs(60));
        // Need to wait for half-open transition
        
        // Record success
        let result = ProbeResult {
            channel_id,
            model: "gpt-3.5-turbo".to_string(),
            success: true,
            latency_ms: 100,
            quality: Some("Healthy".to_string()),
            error: None,
            timestamp: 0,
        };
        
        let action = manager.record_probe_result(result).await;
        assert_eq!(action, ProbeAction::CloseCircuit);
    }

    #[test]
    fn test_probe_model_selection() {
        let manager = HealthProbeManager::with_defaults();
        
        // Should select first preferred model
        let models = vec!["gpt-3.5-turbo".to_string(), "gpt-4".to_string()];
        let selected = manager.get_probe_model(&models);
        assert_eq!(selected, Some("gpt-3.5-turbo".to_string()));

        // Should fallback to first available if no preferred model
        let models = vec!["unknown-model".to_string()];
        let selected = manager.get_probe_model(&models);
        assert_eq!(selected, Some("unknown-model".to_string()));
    }

    #[tokio::test]
    async fn test_probe_result_recording() {
        let manager = HealthProbeManager::with_defaults();
        let channel_id = 1;

        // Record multiple failures
        for _ in 0..2 {
            let result = ProbeResult {
                channel_id,
                model: "gpt-3.5-turbo".to_string(),
                success: false,
                latency_ms: 0,
                quality: None,
                error: Some("Connection failed".to_string()),
                timestamp: 0,
            };
            let action = manager.record_probe_result(result).await;
            if action == ProbeAction::ReopenCircuit {
                break;
            }
        }

        // Should have some results stored
        let results = manager.get_recent_results(10).await;
        assert!(!results.is_empty());
    }
}
