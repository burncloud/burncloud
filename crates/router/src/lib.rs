//! Router module - Main entry point for the Burncloud Router
//!
//! This module provides:
//! - HTTP routing and proxy functionality
//! - AppState management
//! - Configuration loading
//! - Token validation, rate limiting, logging

//! - Dynamic protocol adaptation
//! - Response handling

mod adaptive_limit;
mod adaptor;
pub mod balancer;
pub mod billing;
mod channel_state;
mod circuit_breaker;
mod config;
mod limiter;
mod model_router;

pub mod exchange_rate;
pub mod notification;
pub mod passthrough;
pub mod price_sync;
pub mod pricing_loader;
pub mod response_parser;
pub mod stream_parser;
pub mod token_counter;

mod state;
mod handlers;
mod proxy_logic;

pub use state::*;
pub use handlers::{
    reload_handler, models_handler, health_status_handler, proxy_handler,
};
pub use proxy_logic::handle_response_with_token_parsing;

use crate::state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Response, StatusCode, Uri},
    routing::post,
    Router,
};
use balancer::RoundRobinBalancer;
use burncloud_common::constants::INTERNAL_PREFIX;
use burncloud_common::types::{ChannelType, OpenAIChatRequest};
use burncloud_database::Database;
use burncloud_database_router::{DbRouterLog, RouterDatabase, TokenValidationResult};
use burncloud_database_models::PriceModel;
use channel_state::ChannelStateTracker;
use circuit_breaker::CircuitBreaker;
use config::{AuthType, Group, GroupMember, RouteTarget, RouterConfig, Upstream};
use futures::stream::StreamExt;
use http_body_util::BodyExt;
use limiter::RateLimiter;
use model_router::ModelRouter;
use reqwest::Client;
use std::sync::Arc;
use std::time::Instant;
use stream_parser::StreamingTokenParser;
use token_counter::StreamingTokenCounter;
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use burncloud_common::types::Channel;
use burncloud_database_models::{Price, PriceModel, TieredPrice, TieredPriceModel};
use circuit_breaker::FailureType;
use passthrough::{should_passthrough, PassthroughDecision};
use response_parser::{parse_error_response, parse_rate_limit_info};

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    pub config: Arc<RwLock<RouterConfig>,
    pub db: Arc<Database>,
    pub balancer: Arc<RoundRobinBalancer>,
    pub limiter: Arc<RateLimiter>,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub log_tx: mpsc::Sender<DbRouterLog>,
    pub model_router: Arc<ModelRouter>,
    pub channel_state_tracker: Arc<ChannelStateTracker>,
    pub adaptor_factory: Arc<adaptor::factory::DynamicAdaptorFactory>,
    pub api_version_detector: Arc<adaptor::detector::ApiVersionDetector>
}

