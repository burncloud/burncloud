//! Immutable reference data for the buyer request logs page.

/// A request record shown in the buyer's request log.
///
/// The page intentionally uses a small, static data set while the API-backed
/// log feed is not connected. Keeping the display values here makes the page
/// and its tests deterministic and keeps rendering logic free of formatting
/// assumptions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogEntry {
    pub timestamp: &'static str,
    pub request_id: &'static str,
    pub model: &'static str,
    pub tier: &'static str,
    pub status: &'static str,
    pub ttft: &'static str,
    pub total_latency: &'static str,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cost: &'static str,
    pub region: &'static str,
    pub failover_count: u8,
    pub receipt: &'static str,
    pub enclave: &'static str,
    pub rate_limit_remaining: u32,
}

impl LogEntry {
    /// Returns the aggregate token count for the request.
    pub const fn total_tokens(self) -> u32 {
        self.prompt_tokens + self.completion_tokens
    }

    /// Formats the compact input/output token label used in the table.
    pub fn token_summary(self) -> String {
        format!("{} in / {} out", self.prompt_tokens, self.completion_tokens)
    }

    /// Returns the trace metadata rendered in the detail drawer.
    pub fn trace_metadata(self) -> String {
        format!(
            "{{\"x-burncloud-tier\":\"{}\",\"x-burncloud-region\":\"{}\",\"x-burncloud-request-id\":\"{}\",\"x-ratelimit-remaining\":{},\"failover-count\":{}}}",
            self.tier.to_ascii_lowercase(),
            self.region,
            self.request_id,
            self.rate_limit_remaining,
            self.failover_count
        )
    }

    /// Formats the trace payload with the indentation used by the reference
    /// inspector, while keeping [`Self::trace_metadata`] available for compact
    /// logging and tests.
    pub fn trace_metadata_pretty(self) -> String {
        format!(
            "{{\n  \"x-burncloud-tier\": \"{}\",\n  \"x-burncloud-region\": \"{}\",\n  \"x-burncloud-request-id\": \"{}\",\n  \"x-ratelimit-remaining\": {},\n  \"failover-count\": {}\n}}",
            self.tier.to_ascii_lowercase(),
            self.region,
            self.request_id,
            self.rate_limit_remaining,
            self.failover_count
        )
    }
}

pub const LOG_ENTRIES: [LogEntry; 4] = [
    LogEntry {
        timestamp: "14:28:11.402",
        request_id: "req_98fa01b2",
        model: "DeepSeek V3 (671B)",
        tier: "Standard",
        status: "200 OK",
        ttft: "138 ms",
        total_latency: "442 ms",
        prompt_tokens: 240,
        completion_tokens: 820,
        cost: "$0.000263",
        region: "US-West (Oregon)",
        failover_count: 0,
        receipt: "#RCPT-982A1-NITRO",
        enclave: "Execution completed inside Confidential Computing enclave with zero telemetry retention.",
        rate_limit_remaining: 1198,
    },
    LogEntry {
        timestamp: "14:27:54.110",
        request_id: "req_98fa01b1",
        model: "DeepSeek R1 Reasoning",
        tier: "Performance",
        status: "200 OK",
        ttft: "290 ms",
        total_latency: "1420 ms",
        prompt_tokens: 110,
        completion_tokens: 420,
        cost: "$0.000980",
        region: "US-West (Oregon)",
        failover_count: 0,
        receipt: "#RCPT-982A1-NITRO",
        enclave: "Execution completed inside Confidential Computing enclave with zero telemetry retention.",
        rate_limit_remaining: 1198,
    },
    LogEntry {
        timestamp: "14:26:02.890",
        request_id: "req_98fa01b0",
        model: "GLM-4 Plus",
        tier: "Standard",
        status: "200 OK",
        ttft: "410 ms",
        total_latency: "820 ms",
        prompt_tokens: 520,
        completion_tokens: 310,
        cost: "$0.000798",
        region: "AP-East (Tokyo)",
        failover_count: 1,
        receipt: "#RCPT-982A1-NITRO",
        enclave: "Execution completed inside Confidential Computing enclave with zero telemetry retention.",
        rate_limit_remaining: 1198,
    },
    LogEntry {
        timestamp: "14:24:19.012",
        request_id: "req_98fa01af",
        model: "Qwen 2.5 72B Instruct",
        tier: "Standard",
        status: "200 OK",
        ttft: "165 ms",
        total_latency: "512 ms",
        prompt_tokens: 80,
        completion_tokens: 240,
        cost: "$0.000200",
        region: "EU-Central (Frankfurt)",
        failover_count: 0,
        receipt: "#RCPT-982A1-NITRO",
        enclave: "Execution completed inside Confidential Computing enclave with zero telemetry retention.",
        rate_limit_remaining: 1198,
    },
];

impl LogEntry {
    /// The deterministic rows displayed by the logs page.
    pub const fn reference() -> &'static [Self; 4] {
        &LOG_ENTRIES
    }
}

/// Finds a request by its stable request ID.
pub fn find_log(request_id: &str) -> Option<LogEntry> {
    LOG_ENTRIES
        .iter()
        .find(|entry| entry.request_id == request_id)
        .copied()
}
