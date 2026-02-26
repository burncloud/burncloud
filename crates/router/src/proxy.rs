//! Proxy logic module extracted from router lib.rs
//!
//! This module contains the large proxy_logic function and response handling functions
//!
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Response, StatusCode, Uri},
};
use burncloud_common::types::{Channel, ChannelType};
use burncloud_database_models::{Price, PriceModel, TieredPriceModel};
use circuit_breaker::FailureType;
use config::{AuthType, Group, GroupMember, RouteTarget, RouterConfig, Upstream};
use futures::stream::StreamExt;
use http_body_util::BodyExt;
use passthrough::{should_passthrough, PassthroughDecision};
use response_parser::{parse_error_response, parse_rate_limit_info};

use crate::state::AppState;

