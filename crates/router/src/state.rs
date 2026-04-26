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
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};

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
}
