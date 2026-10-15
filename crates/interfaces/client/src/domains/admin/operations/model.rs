//! Static display model for the administrator emergency operations page.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationsLogLevel {
    Action,
    Failover,
    Optimization,
}

impl OperationsLogLevel {
    pub const fn tone(self) -> &'static str {
        match self {
            Self::Action => "brand",
            Self::Failover => "warning",
            Self::Optimization => "success",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminOperationsLog {
    pub id: &'static str,
    pub time: &'static str,
    pub level: OperationsLogLevel,
    pub category: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub impact: &'static str,
    pub action_taken: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminOperationsModel {
    pub logs: [AdminOperationsLog; 3],
}

impl AdminOperationsModel {
    pub const fn mock() -> Self {
        Self {
            logs: [
                AdminOperationsLog {
                    id: "auto-101",
                    time: "2 mins ago",
                    level: OperationsLogLevel::Action,
                    category: "Capacity",
                    title: "Autopilot added 16x H100 capacity to DeepSeek V3",
                    description: "Demand surge detected in US-West (+34% concurrency). Autopilot provisioned idle supplier nodes without operator intervention.",
                    impact: "p95 latency stabilized from 620ms to 380ms.",
                    action_taken: "Auto-scaled 2 standby worker nodes.",
                },
                AdminOperationsLog {
                    id: "auto-102",
                    time: "18 mins ago",
                    level: OperationsLogLevel::Failover,
                    category: "Failover",
                    title: "Isolated unstable Node ap-hkg-03 due to PCIe thermal warning",
                    description: "Node temperature exceeded 72C. Autopilot drained in-flight requests and rerouted traffic seamlessly to Shenzhen cluster.",
                    impact: "0 customer requests dropped; 100% SLA preserved.",
                    action_taken: "Marked node Degraded and alerted supplier.",
                },
                AdminOperationsLog {
                    id: "auto-103",
                    time: "1 hour ago",
                    level: OperationsLogLevel::Optimization,
                    category: "Economics",
                    title: "Optimized Batch Inference routing for Economy tier",
                    description: "Rerouted 4.2M non-time-critical tokens to off-peak compute nodes.",
                    impact: "Platform gross margin increased by +2.4%.",
                    action_taken: "Updated dynamic batching parameters.",
                },
            ],
        }
    }
}
