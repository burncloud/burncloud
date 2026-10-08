//! Static display model for the administrator overview.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdminEventKind {
    Capacity,
    Failover,
    Economics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminEvent {
    pub kind: AdminEventKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdminOverviewModel {
    pub gmv: &'static str,
    pub demand_share: &'static str,
    pub compute: &'static str,
    pub sla: &'static str,
    pub events: [AdminEvent; 3],
    pub supply_breakdown: [&'static str; 3],
    pub demand_breakdown: [&'static str; 3],
}

impl AdminOverviewModel {
    pub const fn mock() -> Self {
        Self {
            gmv: "$18,450",
            demand_share: "32.4%",
            compute: "420",
            sla: "99.99%",
            events: [
                AdminEvent {
                    kind: AdminEventKind::Capacity,
                },
                AdminEvent {
                    kind: AdminEventKind::Failover,
                },
                AdminEvent {
                    kind: AdminEventKind::Economics,
                },
            ],
            supply_breakdown: ["280 GPUs (66.7%)", "96 GPUs (22.8%)", "44 GPUs (10.5%)"],
            demand_breakdown: ["42,800 tokens / sec", "89.2% (High)", "1,420 Active Teams"],
        }
    }
}
