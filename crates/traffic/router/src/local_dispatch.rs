use super::{
    build_response_with_header, circuit_breaker, get_parser, parse_chunk_or_default,
    parse_rate_limit_info, parse_response_or_default, record_upstream_failure,
    record_upstream_success, state, AppState, Arc, Body, ChannelType, FailureType, Response,
    StatusCode, UnifiedTokenCounter, Upstream, PROTOCOL_OPENAI,
};
use crate::local_attachment::LocalDispatchPermit;
use futures::StreamExt;
use serde_json::Value;

// 两处发送入口共用本机结果处理，避免响应头成功、分块边界和槽位所有权出现分歧。
pub(super) struct LocalExchange {
    state: AppState,
    upstream: Upstream,
    model: String,
    session: String,
    permit: LocalDispatchPermit,
    counter: Arc<UnifiedTokenCounter>,
    started: std::time::Instant,
    deadline: Option<std::time::Instant>,
    outcome: std::cell::Cell<bool>,
    probe_guard: Option<circuit_breaker::CircuitProbeGuard>,
}

impl LocalExchange {
    pub(super) fn new(
        state: &AppState,
        upstream: &Upstream,
        model: &str,
        session: &str,
        permit: LocalDispatchPermit,
        counter: Arc<UnifiedTokenCounter>,
    ) -> Self {
        Self {
            state: state.clone(),
            upstream: upstream.clone(),
            model: model.into(),
            session: session.into(),
            permit,
            counter,
            started: std::time::Instant::now(),
            deadline: None,
            outcome: std::cell::Cell::new(false),
            probe_guard: None,
        }
    }

    fn failure(&self, kind: FailureType, message: &str) {
        self.outcome.set(true);
        let locked = !matches!(kind, FailureType::RateLimited { .. });
        if self.permit.feedback(locked) {
            record_upstream_failure(
                &self.state,
                &self.upstream,
                Some(&self.model),
                kind,
                message,
                &self.session,
            );
        }
    }

    fn success(&self, headers: &reqwest::header::HeaderMap) {
        self.outcome.set(true);
        if !self.permit.feedback(false) {
            return;
        }
        record_upstream_success(
            &self.state,
            &self.upstream,
            Some(&self.model),
            &self.session,
        );
        let info = parse_rate_limit_info(headers, None, PROTOCOL_OPENAI);
        if let Some(learned_limit) = self.state.channel_state_tracker.record_success(
            self.upstream.id.parse().unwrap_or(0),
            Some(&self.model),
            self.started.elapsed().as_millis() as u64,
            info.request_limit,
        ) {
            if let Err(error) = self.state.budget_update_tx.try_send(state::BudgetUpdate {
                channel_id: self.upstream.id.parse().unwrap_or(0),
                learned_limit,
            }) {
                tracing::debug!(%error, "local budget update queue unavailable");
            }
        }
    }

    pub(super) async fn send(
        mut self,
        request: reqwest::RequestBuilder,
        streaming: bool,
        deadline: Option<std::time::Instant>,
        probe_guard: Option<circuit_breaker::CircuitProbeGuard>,
    ) -> Result<Response<Body>, String> {
        self.deadline = deadline;
        self.probe_guard = probe_guard;
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                self.failure(
                    if error.is_timeout() {
                        FailureType::Timeout
                    } else {
                        FailureType::ConnectionError
                    },
                    &error.to_string(),
                );
                return Err(error.to_string());
            }
        };
        let status = response.status();
        let headers = response.headers().clone();
        if status.is_success() && streaming {
            return Ok(self.stream(response, headers));
        }
        if !status.is_success() {
            if let Some(kind) = classify_local_error(status, None, &headers) {
                self.failure(kind, "local upstream rejected endpoint at response headers");
                return Err(format!("local upstream returned {status}"));
            }
        }
        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => {
                self.failure(
                    if error.is_timeout() {
                        FailureType::Timeout
                    } else {
                        FailureType::ConnectionError
                    },
                    &error.to_string(),
                );
                return Err(error.to_string());
            }
        };
        let parsed = serde_json::from_slice::<Value>(&bytes);
        let json = parsed.as_ref().ok();
        if let Some(kind) = classify_local_error(status, json, &headers) {
            self.failure(kind, "local upstream returned an endpoint or service error");
            return Err(format!("local upstream returned {status}"));
        }
        if status.is_success() && json.is_some_and(is_request_error) {
            self.outcome.set(true);
            return Ok(build_response_with_header(
                StatusCode::BAD_REQUEST,
                "content-type",
                "application/json",
                Body::from(bytes),
            ));
        }
        if status.is_success() {
            let value = parsed.map_err(|error| {
                self.failure(FailureType::EmptyResponse, "invalid local JSON response");
                error.to_string()
            })?;
            if !valid_completion(&value) {
                self.failure(
                    FailureType::EmptyResponse,
                    "invalid local completion envelope",
                );
                return Err("invalid local completion envelope".into());
            }
            self.counter.set_from_usage(&parse_response_or_default(
                get_parser(ChannelType::OpenAI).as_ref(),
                &value,
                "local",
            ));
            self.success(&headers);
        }
        self.outcome.set(true);
        // 请求参数错误保持原响应，不写健康失败；取消则随 future 归还许可。
        Ok(build_response_with_header(
            status,
            "content-type",
            "application/json",
            Body::from(bytes),
        ))
    }

    fn stream(
        self,
        response: reqwest::Response,
        headers: reqwest::header::HeaderMap,
    ) -> Response<Body> {
        let stream = futures::stream::try_unfold(
            (
                self,
                response.bytes_stream().boxed(),
                SseDecoder::default(),
                headers,
            ),
            |(exchange, mut body, mut decoder, headers)| async move {
                match body.next().await {
                    Some(Ok(bytes)) => {
                        if let Err(error) = decoder.push(&bytes, &exchange.counter) {
                            if let Some(kind) = error.kind {
                                exchange.failure(kind, &error.message);
                            } else {
                                exchange.outcome.set(true);
                            }
                            return Err(std::io::Error::other(error.message));
                        }
                        Ok(Some((bytes, (exchange, body, decoder, headers))))
                    }
                    Some(Err(error)) => {
                        exchange.failure(
                            if error.is_timeout() {
                                FailureType::Timeout
                            } else {
                                FailureType::ConnectionError
                            },
                            &error.to_string(),
                        );
                        Err(std::io::Error::other(error))
                    }
                    None => {
                        if let Err(error) = decoder.finish(&exchange.counter) {
                            if let Some(kind) = error.kind {
                                exchange.failure(kind, &error.message);
                            } else {
                                exchange.outcome.set(true);
                            }
                            return Err(std::io::Error::other(error.message));
                        }
                        exchange.success(&headers);
                        // exchange 持有唯一许可；EOF、错误或丢弃响应体均只释放一次。
                        Ok(None)
                    }
                }
            },
        );
        build_response_with_header(
            StatusCode::OK,
            "content-type",
            "text/event-stream",
            Body::from_stream(stream),
        )
    }
}

impl Drop for LocalExchange {
    fn drop(&mut self) {
        // 总预算取消已经发送的上游时也需反馈超时；普通下游取消不算模型故障。
        if !self.outcome.get()
            && self
                .deadline
                .is_some_and(|at| std::time::Instant::now() >= at)
        {
            self.failure(
                FailureType::Timeout,
                "local request deadline exceeded after send",
            );
        }
    }
}

fn classify_local_error(
    status: StatusCode,
    body: Option<&Value>,
    headers: &reqwest::header::HeaderMap,
) -> Option<FailureType> {
    let error = body.and_then(|v| v.get("error"));
    let code = error
        .and_then(|v| v.get("code").or_else(|| v.get("type")))
        .and_then(Value::as_str);
    if code == Some("model_not_found") {
        return Some(FailureType::ModelNotFound);
    }
    if status.is_success() && body.is_some_and(is_request_error) {
        return None;
    }

    match status {
        StatusCode::TOO_MANY_REQUESTS => Some(FailureType::RateLimited {
            scope: circuit_breaker::RateLimitScope::Model,
            retry_after: headers
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok()),
        }),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Some(FailureType::AuthFailed),
        StatusCode::PAYMENT_REQUIRED => Some(FailureType::PaymentRequired),
        StatusCode::NOT_FOUND
        | StatusCode::METHOD_NOT_ALLOWED
        | StatusCode::UNSUPPORTED_MEDIA_TYPE => Some(FailureType::ServerError),
        _ if status.is_server_error() || status.is_redirection() => Some(FailureType::ServerError),
        _ if status.is_success()
            && (error.is_some()
                || body
                    .is_some_and(|v| v.get("type").and_then(Value::as_str) == Some("error"))) =>
        {
            Some(FailureType::ServerError)
        }
        _ => None,
    }
}

fn valid_completion(value: &Value) -> bool {
    value
        .get("choices")
        .and_then(Value::as_array)
        .is_some_and(|choices| {
            !choices.is_empty()
                && choices.iter().all(|choice| {
                    choice.get("message").is_some_and(|message| {
                        message.get("content").is_some_and(Value::is_string)
                            || message.get("refusal").is_some_and(Value::is_string)
                            || message
                                .get("tool_calls")
                                .and_then(Value::as_array)
                                .is_some_and(|calls| !calls.is_empty())
                            || message.get("function_call").is_some_and(Value::is_object)
                    }) || choice.get("text").is_some_and(Value::is_string)
                })
        })
}

fn is_request_error(value: &Value) -> bool {
    value.get("error").is_some_and(|error| {
        ["code", "type"].iter().any(|key| {
            matches!(
                error.get(*key).and_then(Value::as_str),
                Some(
                    "context_length_exceeded"
                        | "content_policy_violation"
                        | "content_filter"
                        | "invalid_request_error"
                )
            )
        }) && error.get("code").and_then(Value::as_str) != Some("model_not_found")
    })
}

#[derive(Debug)]
struct StreamFailure {
    message: String,
    kind: Option<FailureType>,
}
impl From<String> for StreamFailure {
    fn from(message: String) -> Self {
        Self {
            message,
            kind: Some(FailureType::ServerError),
        }
    }
}
impl From<&str> for StreamFailure {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}

fn stream_failure(value: &Value) -> StreamFailure {
    let kind = if is_request_error(value) {
        None
    } else if value.pointer("/error/code").and_then(Value::as_str) == Some("rate_limit_exceeded") {
        Some(FailureType::RateLimited {
            scope: circuit_breaker::RateLimitScope::Model,
            retry_after: None,
        })
    } else {
        classify_local_error(
            StatusCode::OK,
            Some(value),
            &reqwest::header::HeaderMap::new(),
        )
        .or(Some(FailureType::ServerError))
    };
    StreamFailure {
        message: "local SSE upstream error".into(),
        kind,
    }
}

#[derive(Default)]
struct SseDecoder {
    line: Vec<u8>,
    data: Vec<u8>,
    error_event: bool,
    seen: bool,
    complete: bool,
    after_cr: bool,
}

impl SseDecoder {
    fn push(&mut self, chunk: &[u8], counter: &UnifiedTokenCounter) -> Result<(), StreamFailure> {
        for &byte in chunk {
            if self.after_cr && byte == b'\n' {
                self.after_cr = false;
                continue;
            }
            self.after_cr = byte == b'\r';
            if byte == b'\n' || byte == b'\r' {
                self.line(counter)?;
            } else {
                self.line.push(byte);
                if self.line.len() + self.data.len() > 1024 * 1024 {
                    return Err("local SSE event exceeds 1 MiB".into());
                }
            }
        }
        Ok(())
    }

    fn line(&mut self, counter: &UnifiedTokenCounter) -> Result<(), StreamFailure> {
        if self.line.is_empty() {
            return self.event(counter);
        }
        if let Some(data) = self.line.strip_prefix(b"data:") {
            if !self.data.is_empty() {
                self.data.push(b'\n');
            }
            self.data
                .extend_from_slice(data.strip_prefix(b" ").unwrap_or(data));
        } else if self.line.starts_with(b"event:") {
            self.error_event = self.line[6..].trim_ascii() == b"error";
        }
        self.line.clear();
        Ok(())
    }

    fn event(&mut self, counter: &UnifiedTokenCounter) -> Result<(), StreamFailure> {
        if self.error_event && self.data.is_empty() {
            return Err("local SSE service error event".into());
        }
        if self.data.is_empty() {
            return Ok(());
        }
        let data = self.data.trim_ascii();
        if data == b"[DONE]" {
            self.complete = true;
        } else {
            let json: Value =
                serde_json::from_slice(data).map_err(|e| format!("invalid local SSE JSON: {e}"))?;
            if self.error_event
                || json.get("error").is_some()
                || json.get("type").and_then(Value::as_str) == Some("error")
            {
                return Err(stream_failure(&json));
            }
            let choices = json
                .get("choices")
                .and_then(Value::as_array)
                .ok_or("missing local SSE choices")?;
            for choice in choices {
                if !choice.get("delta").is_some_and(Value::is_object)
                    && !choice.get("text").is_some_and(Value::is_string)
                {
                    return Err("invalid local SSE choice".into());
                }
                self.seen = true;
                self.complete |= choice.get("finish_reason").is_some_and(|v| !v.is_null());
            }
            if let Some(usage) = parse_chunk_or_default(
                get_parser(ChannelType::OpenAI).as_ref(),
                &format!("data: {}\n\n", String::from_utf8_lossy(data)),
                "local",
            ) {
                counter.set_from_usage(&usage);
            }
        }
        self.data.clear();
        self.error_event = false;
        Ok(())
    }

    fn finish(&mut self, counter: &UnifiedTokenCounter) -> Result<(), StreamFailure> {
        if !self.line.is_empty() {
            self.line(counter)?;
        }
        self.event(counter)?;
        if self.seen && self.complete {
            Ok(())
        } else {
            Err("incomplete local SSE response".into())
        }
    }
}

#[cfg(test)]
#[path = "local_dispatch_tests.rs"]
mod tests;
