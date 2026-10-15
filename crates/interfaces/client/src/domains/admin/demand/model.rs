//! Static display model for the administrator token demand page.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DemandAccount {
    pub tenant: &'static str,
    pub tier: &'static str,
    pub tokens: &'static str,
    pub spend: &'static str,
    pub model: &'static str,
    pub model_spend: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminDemandModel {
    pub throughput: &'static str,
    pub top_accounts: &'static str,
    pub low_latency_share: &'static str,
    pub prompt_completion_tokens: &'static str,
    pub accounts: [DemandAccount; 3],
}

impl AdminDemandModel {
    pub const fn mock() -> Self {
        Self {
            throughput: "42,800",
            top_accounts: "1,420",
            low_latency_share: "58.2%",
            prompt_completion_tokens: "1,420",
            accounts: [
                DemandAccount {
                    tenant: "HyperScale AI Labs (US-West)",
                    tier: "Performance tier",
                    tokens: "148.2M",
                    spend: "$4,280.00",
                    model: "DeepSeek V3 (671B)",
                    model_spend: "$18,400.00",
                },
                DemandAccount {
                    tenant: "Apex Robotics Co.",
                    tier: "Standard tier",
                    tokens: "84.1M",
                    spend: "$2,140.50",
                    model: "DeepSeek R1",
                    model_spend: "$6,200.00",
                },
                DemandAccount {
                    tenant: "FinTech Intelligence Inc.",
                    tier: "Performance tier",
                    tokens: "52.0M",
                    spend: "$1,920.00",
                    model: "Qwen 2.5 72B",
                    model_spend: "$12,850.00",
                },
            ],
        }
    }
}
