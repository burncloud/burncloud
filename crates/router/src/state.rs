//! Application state for the router

use crate::adaptor;
use crate::balancer::RoundRobinBalancer;
use crate::channel_state::ChannelStateTracker;
use crate::circuit_breaker::CircuitBreaker;
use crate::config::RouterConfig;
use crate::limiter::RateLimiter;
use crate::model_router::ModelRouter;
use burncloud_database::Database;
use burncloud_database_router::DbRouterLog;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    pub config: Arc<RwLock<RouterConfig>>,
    pub db: Arc<Database>,
    pub balancer: Arc<RoundRobinBalancer>,
    pub limiter: Arc<RateLimiter>,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub log_tx: mpsc::Sender<DbRouterLog>,
    pub model_router: Arc<ModelRouter>,
    pub channel_state_tracker: Arc<ChannelStateTracker>,
    pub adaptor_factory: Arc<adaptor::factory::DynamicAdaptorFactory>,
    pub api_version_detector: Arc<adaptor::detector::ApiVersionDetector>,
}
