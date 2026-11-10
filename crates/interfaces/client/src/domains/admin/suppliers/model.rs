//! Reference supplier directory shown in the administrator console.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminSupplier {
    pub name: &'static str,
    pub tier: &'static str,
    pub connected_gpus: &'static str,
    pub uptime_sla: &'static str,
    pub revenue_share: &'static str,
    pub earnings_30d: &'static str,
    pub status: &'static str,
}

pub const SUPPLIERS: [AdminSupplier; 4] = [
    AdminSupplier {
        name: "Silicon Bay Data Centers LLC",
        tier: "L4 Strategic",
        connected_gpus: "128x H100 SXM5",
        uptime_sla: "99.98%",
        revenue_share: "85%",
        earnings_30d: "$118,400",
        status: "Online",
    },
    AdminSupplier {
        name: "Frankfurt EuroCompute GmbH",
        tier: "L3 Professional",
        connected_gpus: "64x A100 80GB",
        uptime_sla: "99.94%",
        revenue_share: "80%",
        earnings_30d: "$52,100",
        status: "Online",
    },
    AdminSupplier {
        name: "Tokyo Cloud Matrix Ltd",
        tier: "L3 Professional",
        connected_gpus: "48x H100 SXM5",
        uptime_sla: "99.91%",
        revenue_share: "80%",
        earnings_30d: "$44,800",
        status: "Online",
    },
    AdminSupplier {
        name: "Nordic AI Infra AS",
        tier: "L2 Verified",
        connected_gpus: "24x RTX 4090 Pool",
        uptime_sla: "99.72%",
        revenue_share: "75%",
        earnings_30d: "$12,400",
        status: "Online",
    },
];
