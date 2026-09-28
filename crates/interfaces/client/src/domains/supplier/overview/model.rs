//! Static display model for the supplier overview.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeStatus {
    Online,
    Degraded,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupplierNode {
    pub name: &'static str,
    pub region: &'static str,
    pub site: &'static str,
    pub hardware: &'static str,
    pub memory: &'static str,
    pub vram: &'static str,
    pub interconnect: &'static str,
    pub model: &'static str,
    pub utilization: f32,
    pub temperature_c: u8,
    pub earnings: f64,
    pub status: NodeStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SupplierOverviewModel {
    pub today_earnings: f64,
    pub earnings_trend: &'static str,
    pub active_gpu_count: u32,
    pub total_gpu_count: u32,
    pub cluster_count: u32,
    pub gpu_utilization: f32,
    pub peak_compute: f32,
    pub tokens_today: f32,
    pub alert_node: &'static str,
    pub nodes: Vec<SupplierNode>,
    pub install_command: &'static str,
}

impl SupplierOverviewModel {
    pub fn mock() -> Self {
        Self {
            today_earnings: 382.40,
            earnings_trend: "+12.4% vs yesterday",
            active_gpu_count: 24,
            total_gpu_count: 28,
            cluster_count: 3,
            gpu_utilization: 74.0,
            peak_compute: 88.6,
            tokens_today: 24.2,
            alert_node: "HKG-Edge-RTX-Pool",
            nodes: vec![
                SupplierNode {
                    name: "SJC-Pod-01-Rack4",
                    region: "us-west-sjc",
                    site: "Silicon-Bay-A",
                    hardware: "8 x NVIDIA H100 SXM5 80GB",
                    memory: "640 GB VRAM • NVLink 900 GB/s",
                    vram: "640 GB High-Bandwidth",
                    interconnect: "NVLink 900 GB/s",
                    model: "DeepSeek V3 (Standard Tier)",
                    utilization: 91.4,
                    temperature_c: 58,
                    earnings: 184.20,
                    status: NodeStatus::Online,
                },
                SupplierNode {
                    name: "SJC-Pod-01-Rack5",
                    region: "us-west-sjc",
                    site: "Silicon-Bay-A",
                    hardware: "8 x NVIDIA H100 SXM5 80GB",
                    memory: "640 GB VRAM • NVLink 900 GB/s",
                    vram: "640 GB High-Bandwidth",
                    interconnect: "NVLink 900 GB/s",
                    model: "DeepSeek R1 (Performance Tier)",
                    utilization: 88.6,
                    temperature_c: 61,
                    earnings: 178.60,
                    status: NodeStatus::Online,
                },
                SupplierNode {
                    name: "FRA-DC2-Compute-08",
                    region: "eu-central-fra",
                    site: "Frankfurt-EuroNode",
                    hardware: "8 x NVIDIA A100-SXM4 80GB",
                    memory: "640 GB VRAM • NVLink 600 GB/s",
                    vram: "640 GB High-Bandwidth",
                    interconnect: "NVLink 600 GB/s",
                    model: "Qwen 2.5 72B (Standard Tier)",
                    utilization: 74.2,
                    temperature_c: 64,
                    earnings: 79.40,
                    status: NodeStatus::Online,
                },
                SupplierNode {
                    name: "HKG-Edge-RTX-Pool",
                    region: "ap-east-hkg",
                    site: "HKG-Community-01",
                    hardware: "4 x NVIDIA RTX 4090 24GB",
                    memory: "96 GB VRAM • PCIe 4.0 x16",
                    vram: "96 GB High-Bandwidth",
                    interconnect: "PCIe 4.0 x16",
                    model: "Llama 3.3 70B Quantized (Economy)",
                    utilization: 42.0,
                    temperature_c: 72,
                    earnings: 18.20,
                    status: NodeStatus::Degraded,
                },
            ],
            install_command:
                "curl -sSL https://burncloud.io/install.sh | bash -s -- --token=bc_node_auth_demo",
        }
    }
}
