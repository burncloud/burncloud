//! Static display model for the administrator capacity and autoscale page.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapacityStatus {
    Healthy,
    Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapacityModelRow {
    pub model: &'static str,
    pub provisioned: &'static str,
    pub active: &'static str,
    pub headroom: &'static str,
    pub latency: &'static str,
    pub action: &'static str,
    pub status: CapacityStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminCapacityModel {
    pub safety_margin: &'static str,
    pub peak_throughput: &'static str,
    pub replicas: &'static str,
    pub latency_target: &'static str,
    pub models: [CapacityModelRow; 4],
}

impl AdminCapacityModel {
    pub const fn mock() -> Self {
        Self {
            safety_margin: "34.2%",
            peak_throughput: "48,200",
            replicas: "58",
            latency_target: "< 200ms",
            models: [
                CapacityModelRow {
                    model: "DeepSeek V3 (671B MoE)",
                    provisioned: "32,000 tok/s",
                    active: "24,800 tok/s",
                    headroom: "22.5%",
                    latency: "148 ms",
                    action: "Auto-scaled +2 nodes",
                    status: CapacityStatus::Healthy,
                },
                CapacityModelRow {
                    model: "DeepSeek R1 Reasoning",
                    provisioned: "18,000 tok/s",
                    active: "12,400 tok/s",
                    headroom: "31.1%",
                    latency: "380 ms",
                    action: "Stable",
                    status: CapacityStatus::Healthy,
                },
                CapacityModelRow {
                    model: "Qwen 2.5 72B Instruct",
                    provisioned: "14,000 tok/s",
                    active: "8,900 tok/s",
                    headroom: "36.4%",
                    latency: "190 ms",
                    action: "Stable",
                    status: CapacityStatus::Healthy,
                },
                CapacityModelRow {
                    model: "GLM-4 Plus",
                    provisioned: "8,000 tok/s",
                    active: "6,800 tok/s",
                    headroom: "15.0%",
                    latency: "780 ms",
                    action: "Standby node provisioning",
                    status: CapacityStatus::Degraded,
                },
            ],
        }
    }
}
