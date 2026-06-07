//! Application state for the router

use crate::adaptor;
use crate::affinity::AffinityCache;
use crate::balancer::RoundRobinBalancer;
use crate::channel_state::ChannelStateTracker;
use crate::circuit_breaker::CircuitBreaker;
use crate::exchange_rate::ExchangeRateService;
use crate::limiter::RateLimiter;
use crate::model_router::ModelRouter;
use crate::price_sync::SyncResult;
use crate::rate_budget::InMemoryBudget;
use crate::scheduler::SchedulerPolicyMap;
use burncloud_database::Database;
use burncloud_database_router::{RouterLog, RouterRequestLog, StoragePolicy};
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
    pub db: Arc<Database>,
    pub balancer: Arc<RoundRobinBalancer>,
    pub limiter: Arc<RateLimiter>,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub log_tx: mpsc::Sender<RouterLog>,
    /// Channel for async request log writes (router_request_logs table).
    /// Capacity matches log_tx for consistent throughput.
    pub request_log_tx: mpsc::Sender<RouterRequestLog>,
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
    /// Billing strict mode: when true, preflight PriceNotFound returns 400;
    /// when false, only warns and allows the request through.
    pub billing_strict: bool,
    /// Counter for requests rejected by billing preflight (PriceNotFound in strict mode).
    pub billing_preflight_rejected_count: Arc<AtomicU64>,
    /// Counter for post-settle price-missing hits (PriceNotFound after request completed).
    /// A healthy system should see this remain at 0.
    pub billing_post_settle_price_missing_count: Arc<AtomicU64>,
    /// AIMD → InMemoryBudget feedback channel (capacity=1, latest-wins debounce).
    pub budget_update_tx: mpsc::Sender<BudgetUpdate>,
    /// Storage policy for request logs: full (complete bodies), summary (metadata only), none.
    /// Default is summary for production; full for dev/debug.
    pub request_log_storage_policy: StoragePolicy,
}
