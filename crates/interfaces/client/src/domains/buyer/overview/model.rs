//! Display model for the buyer overview. The values intentionally remain demo
//! data until billing and usage APIs are migrated.

#[derive(Clone, Debug, PartialEq)]
pub struct OverviewModel {
    pub today_spend: f64,
    pub prepaid_balance: Option<f64>,
    pub api_availability: Option<f64>,
    pub tokens_today: Option<u64>,
    pub models: Vec<ModelUsage>,
    pub recent_activity: Vec<ActivityEvent>,
}

/// A localized metric projected by the overview page.
#[derive(Clone, Debug, PartialEq)]
pub struct Metric {
    pub label: String,
    pub value: String,
    pub detail: String,
    pub trend: Option<String>,
    pub trend_positive: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelUsage {
    pub name: &'static str,
    pub family: &'static str,
    pub tier: &'static str,
    pub tokens: u64,
    pub cost: f64,
    pub latency_ms: u32,
    pub status: ServiceStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Activity {
    pub title: &'static str,
    pub detail: &'static str,
    pub time: &'static str,
}

/// Public name used by the page contract for an activity stream entry.
pub type ActivityEvent = Activity;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unknown,
}

impl OverviewModel {
    pub fn mock() -> Self {
        Self {
            today_spend: 14.28,
            prepaid_balance: Some(128.50),
            api_availability: Some(99.99),
            tokens_today: Some(1_840_000),
            models: vec![
                ModelUsage {
                    name: "DeepSeek V3 (671B MoE)",
                    family: "DeepSeek",
                    tier: "Standard",
                    tokens: 1_120_400,
                    cost: 0.28,
                    latency_ms: 380,
                    status: ServiceStatus::Healthy,
                },
                ModelUsage {
                    name: "DeepSeek R1 Reasoning",
                    family: "DeepSeek",
                    tier: "Performance",
                    tokens: 410_200,
                    cost: 0.89,
                    latency_ms: 620,
                    status: ServiceStatus::Healthy,
                },
                ModelUsage {
                    name: "Qwen 2.5 72B Instruct",
                    family: "Qwen",
                    tier: "Standard",
                    tokens: 310_000,
                    cost: 0.18,
                    latency_ms: 410,
                    status: ServiceStatus::Healthy,
                },
            ],
            recent_activity: vec![
                Activity {
                    title: "Prepaid balance top-up completed ($100.00)",
                    detail: "Receipt #REC-8921 generated. Payment method: Visa ending in 4242.",
                    time: "12 mins ago",
                },
                Activity {
                    title: "Sub-150ms smart fallback verified for DeepSeek V3",
                    detail: "BurnCloud automatically provisioned additional capacity in US-West cluster.",
                    time: "1 hour ago",
                },
                Activity {
                    title: "New API Key \"Production Kubernetes Cluster\" generated",
                    detail: "Associated with Standard & Performance tiers.",
                    time: "1 day ago",
                },
            ],
        }
    }
}
