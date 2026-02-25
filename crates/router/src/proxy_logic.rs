//! Proxy logic module extracted from router lib.rs
//!
//! This module contains the large proxy_logic function and response handling functions
//! that from the massive lib.rs file.

use axum::{
    body::Body,
    http::{HeaderMap, Method, Response, StatusCode, Uri},
};
use burncloud_common::types::ChannelType;
use burncloud_database_models::PriceModel;
use circuit_breaker::FailureType;
use config::{AuthType, Group, GroupMember, RouteTarget, RouterConfig, Upstream};
use futures::stream::StreamExt;
use passthrough::{should_passthrough, PassthroughDecision};
use response_parser::{parse_error_response, parse_rate_limit_info};
use std::sync::Arc;
use std::time::Instant;
use crate::state::AppState;
use crate::token_counter::StreamingTokenCounter;

/// Handle streaming response with token parsing for OpenAI protocol
pub fn handle_response_with_token_parsing(
    resp: reqwest::Response,
    token_counter: &Arc<StreamingTokenCounter>,
    protocol: &str,
) -> Response {
    let status = resp.status();
    let mut response_builder = Response::builder().status(status);

    if let Some(headers_mut) = response_builder.headers_mut() {
        for (k, v) in resp.headers() {
            headers_mut.insert(k, v.clone());
        }
    }

    let counter_clone = Arc::clone(token_counter);
    let protocol = protocol.to_string();
    let stream = resp.bytes_stream();

    let mapped_stream = stream.map(move |chunk_result| match chunk_result {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);

            // Parse token usage from streaming response
            match protocol.as_str() {
                "claude" => {
                    StreamingTokenParser::parse_anthropic_chunk(&text, &counter_clone);
                }
                "gemini" | "vertex" => {
                    StreamingTokenParser::parse_gemini_chunk(&text, &counter_clone);
                }
                _ => {
                    StreamingTokenParser::parse_openai_chunk(&text, &counter_clone);
                }
            }

            Ok(bytes)
        }
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    });

    let body = Body::from_stream(mapped_stream);

    response_builder
        .body(body)
        .unwrap_or_else(|_| Response::new(Body::empty()))
}
