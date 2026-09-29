//! Static display model for the supplier earnings page.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupplierEarningRow {
    pub node: &'static str,
    pub hardware: &'static str,
    pub model: &'static str,
    pub tokens: &'static str,
    pub revenue_share: &'static str,
    pub earnings: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupplierEarningsModel {
    pub today_earnings: &'static str,
    pub today_trend: &'static str,
    pub monthly_earnings: &'static str,
    pub monthly_period: &'static str,
    pub total_tokens: &'static str,
    pub valid_tokens: &'static str,
    pub effective_rate: &'static str,
    pub effective_rate_detail: &'static str,
    pub rows: [SupplierEarningRow; 4],
}

pub const REFERENCE_EARNINGS: SupplierEarningsModel = SupplierEarningsModel {
    today_earnings: "$382.40",
    today_trend: "+14.2% vs yesterday",
    monthly_earnings: "$8,420.50",
    monthly_period: "Aug 1 - Aug 22",
    total_tokens: "482.1M",
    valid_tokens: "99.98% valid tokens",
    effective_rate: "$1.94",
    effective_rate_detail: "8x H100 effective rate",
    rows: [
        SupplierEarningRow {
            node: "SJC-Pod-01-Rack4",
            hardware: "8x H100 SXM5",
            model: "DeepSeek V3 (671B)",
            tokens: "184.2M",
            revenue_share: "80%",
            earnings: "$184.20",
        },
        SupplierEarningRow {
            node: "SJC-Pod-01-Rack5",
            hardware: "8x H100 SXM5",
            model: "DeepSeek R1",
            tokens: "92.4M",
            revenue_share: "80%",
            earnings: "$178.60",
        },
        SupplierEarningRow {
            node: "FRA-DC2-Compute-08",
            hardware: "8x A100 80GB",
            model: "Qwen 2.5 72B",
            tokens: "142.1M",
            revenue_share: "80%",
            earnings: "$79.40",
        },
        SupplierEarningRow {
            node: "HKG-Edge-RTX-Pool",
            hardware: "4x RTX 4090",
            model: "Llama 3.3 70B",
            tokens: "63.4M",
            revenue_share: "75%",
            earnings: "$18.20",
        },
    ],
};

impl SupplierEarningsModel {
    pub const fn reference() -> &'static Self {
        &REFERENCE_EARNINGS
    }
}
