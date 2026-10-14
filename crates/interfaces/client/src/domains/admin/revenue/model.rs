//! Static display model for the administrator revenue page.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminRevenueRow {
    pub model: &'static str,
    pub tokens: &'static str,
    pub gmv: &'static str,
    pub supplier_payout: &'static str,
    pub net_revenue: &'static str,
    pub net_margin: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminRevenueModel {
    pub gmv: &'static str,
    pub supplier_payouts: &'static str,
    pub net_revenue: &'static str,
    pub prepaid_credits: &'static str,
    pub rows: [AdminRevenueRow; 4],
}

pub const REFERENCE_REVENUE: AdminRevenueModel = AdminRevenueModel {
    gmv: "$384,200",
    supplier_payouts: "$259,720",
    net_revenue: "$124,480",
    prepaid_credits: "$842,100",
    rows: [
        AdminRevenueRow {
            model: "DeepSeek V3 (671B)",
            tokens: "1,420.0M",
            gmv: "$148,200",
            supplier_payout: "$98,400",
            net_revenue: "$49,800",
            net_margin: "33.6%",
        },
        AdminRevenueRow {
            model: "DeepSeek R1 Reasoning",
            tokens: "680.0M",
            gmv: "$112,400",
            supplier_payout: "$74,200",
            net_revenue: "$38,200",
            net_margin: "34.0%",
        },
        AdminRevenueRow {
            model: "Qwen 2.5 72B",
            tokens: "420.0M",
            gmv: "$64,100",
            supplier_payout: "$44,800",
            net_revenue: "$19,300",
            net_margin: "30.1%",
        },
        AdminRevenueRow {
            model: "Claude 3.5 Sonnet Pass-Through",
            tokens: "140.0M",
            gmv: "$59,500",
            supplier_payout: "$42,320",
            net_revenue: "$17,180",
            net_margin: "28.9%",
        },
    ],
};

impl AdminRevenueModel {
    pub const fn reference() -> &'static Self {
        &REFERENCE_REVENUE
    }
}
