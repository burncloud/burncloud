//! Fixed display data for buyer usage analytics.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UsageMetricKind {
    Tokens,
    Cost,
    Requests,
    Latency,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UsageMetric {
    pub kind: UsageMetricKind,
    pub value: &'static str,
    pub unit: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UsageBarTone {
    Dark,
    Indigo,
    Green,
    Amber,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UsageBreakdown {
    pub name: &'static str,
    pub tokens: &'static str,
    pub percent: u8,
    pub cost: &'static str,
    pub tone: UsageBarTone,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UsageRow {
    pub model: &'static str,
    pub input_tokens: &'static str,
    pub output_tokens: &'static str,
    pub requests: &'static str,
    pub latency: &'static str,
    pub cost: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UsageModel {
    pub metrics: [UsageMetric; 4],
    pub breakdown: [UsageBreakdown; 4],
    pub rows: [UsageRow; 3],
}

pub const REFERENCE_USAGE: UsageModel = UsageModel {
    metrics: [
        UsageMetric {
            kind: UsageMetricKind::Tokens,
            value: "14.82M",
            unit: Some("tokens"),
        },
        UsageMetric {
            kind: UsageMetricKind::Cost,
            value: "$114.60",
            unit: None,
        },
        UsageMetric {
            kind: UsageMetricKind::Requests,
            value: "182,490",
            unit: None,
        },
        UsageMetric {
            kind: UsageMetricKind::Latency,
            value: "142 ms",
            unit: None,
        },
    ],
    breakdown: [
        UsageBreakdown {
            name: "DeepSeek V3",
            tokens: "8.4M",
            percent: 57,
            cost: "$2.35",
            tone: UsageBarTone::Dark,
        },
        UsageBreakdown {
            name: "DeepSeek R1",
            tokens: "3.8M",
            percent: 26,
            cost: "$8.32",
            tone: UsageBarTone::Indigo,
        },
        UsageBreakdown {
            name: "Qwen 2.5 72B",
            tokens: "1.9M",
            percent: 13,
            cost: "$1.33",
            tone: UsageBarTone::Green,
        },
        UsageBreakdown {
            name: "Claude 3.5 Sonnet",
            tokens: "0.72M",
            percent: 4,
            cost: "$10.80",
            tone: UsageBarTone::Amber,
        },
    ],
    rows: [
        UsageRow {
            model: "DeepSeek V3 (Standard)",
            input_tokens: "2,800,000",
            output_tokens: "5,600,000",
            requests: "114,200",
            latency: "148 ms",
            cost: "$2.35",
        },
        UsageRow {
            model: "DeepSeek R1 (Performance)",
            input_tokens: "1,200,000",
            output_tokens: "2,600,000",
            requests: "38,100",
            latency: "380 ms",
            cost: "$8.32",
        },
        UsageRow {
            model: "Qwen 2.5 72B (Standard)",
            input_tokens: "650,000",
            output_tokens: "1,250,000",
            requests: "22,090",
            latency: "190 ms",
            cost: "$1.33",
        },
    ],
};

impl UsageModel {
    pub const fn reference() -> &'static Self {
        &REFERENCE_USAGE
    }
}
