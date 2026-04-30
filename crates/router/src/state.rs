//! Application state for the router

use crate::adaptor;
use crate::affinity::AffinityCache;
use crate::balancer::RoundRobinBalancer;
use crate::channel_state::ChannelStateTracker;
use crate::circuit_breaker::CircuitBreaker;
use crate::config::RouterConfig;
use crate::exchange_rate::ExchangeRateService;
use crate::limiter::RateLimiter;
use crate::model_router::ModelRouter;
use crate::price_sync::SyncResult;
use crate::rate_budget::InMemoryBudget;
use crate::scheduler::SchedulerPolicyMap;
use burncloud_database::Database;
use burncloud_database_router::RouterLog;
use burncloud_service_billing::{CostCalculator, PriceCache};
use reqwest::Client;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};

/// AIMD → InMemoryBudget feedback message. When the adaptive limiter learns a
/// new limit, this update is sent via a capacity-1 mpsc channel so the budget
/// can be reconfigured asynchronously (audit decision D6/D10 — no lock
/// contention, natural debounce).
pub struct BudgetUpdate {
    pub channel_id: i32,
    pub learned_limit: u32,
}

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    pub config: Arc<RwLock<RouterConfig>>,
    pub db: Arc<Database>,
    pub balancer: Arc<RoundRobinBalancer>,
    pub limiter: Arc<RateLimiter>,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub log_tx: mpsc::Sender<RouterLog>,
    pub model_router: Arc<ModelRouter>,
    pub channel_state_tracker: Arc<ChannelStateTracker>,
    pub adaptor_factory: Arc<adaptor::factory::DynamicAdaptorFactory>,
    pub api_version_detector: Arc<adaptor::detector::ApiVersionDetector>,
    pub price_cache: PriceCache,
    pub cost_calculator: CostCalculator,
    /// Sends force-sync requests to the background price sync task.
    /// The task responds via the enclosed oneshot sender.
    pub force_sync_tx: mpsc::Sender<oneshot::Sender<SyncResult>>,
    pub exchange_rate_service: Arc<ExchangeRateService>,
    pub scheduler_policies: Arc<RwLock<SchedulerPolicyMap>>,
    /// L3 Affinity flow cache (HRW + dual TTL). MVP: shared across all groups.
    pub affinity_cache: Arc<AffinityCache>,
    /// L2 Shaper budget backend. MVP: in-memory, single-instance.
    /// Phase 4 swaps in a Redis-backed impl behind the same `BudgetBackend` trait.
    pub rate_budget: Arc<InMemoryBudget>,
    /// L2 Shaper fail-open counter — incremented every time the failover loop
    /// admits a request through an *unconfigured* channel (rpm_cap = NULL).
    /// Exposed via `/router/status` so admins can spot silently-permissive
    /// channels (audit FM2 — fail-open silent failure).
    pub fail_open_count: Arc<AtomicU64>,
    /// AIMD → InMemoryBudget feedback channel (capacity=1, latest-wins debounce).
    pub budget_update_tx: mpsc::Sender<BudgetUpdate>,
}
