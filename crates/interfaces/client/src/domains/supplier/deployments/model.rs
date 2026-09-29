//! Static display model for the supplier Autopilot deployment page.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeploymentStatus {
    Healthy,
    Degraded,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupplierDeployment {
    pub model: &'static str,
    pub node: &'static str,
    pub runtime: &'static str,
    pub throughput: &'static str,
    pub score: &'static str,
    pub status: DeploymentStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SupplierDeploymentsModel {
    pub deployments: Vec<SupplierDeployment>,
}

impl SupplierDeploymentsModel {
    pub fn mock() -> Self {
        Self {
            deployments: vec![
                SupplierDeployment {
                    model: "DeepSeek V3 (671B MoE)",
                    node: "SJC-Pod-01-Rack4 (8x H100)",
                    runtime: "TP=8 / FP8 FlashInfer",
                    throughput: "85.2 tokens/s",
                    score: "99.8 / 100",
                    status: DeploymentStatus::Healthy,
                },
                SupplierDeployment {
                    model: "DeepSeek R1 Reasoning",
                    node: "SJC-Pod-01-Rack5 (8x H100)",
                    runtime: "TP=8 / DeepSeek-VLLM",
                    throughput: "56.4 tokens/s",
                    score: "99.4 / 100",
                    status: DeploymentStatus::Healthy,
                },
                SupplierDeployment {
                    model: "Qwen 2.5 72B Instruct",
                    node: "FRA-DC2-Compute-08 (8x A100)",
                    runtime: "TP=8 / vLLM v0.6.2",
                    throughput: "72.0 tokens/s",
                    score: "98.9 / 100",
                    status: DeploymentStatus::Healthy,
                },
                SupplierDeployment {
                    model: "Llama 3.3 70B Quantized",
                    node: "HKG-Edge-RTX-Pool (4x 4090)",
                    runtime: "TP=4 / AWQ INT4",
                    throughput: "42.1 tokens/s",
                    score: "92.1 / 100",
                    status: DeploymentStatus::Degraded,
                },
            ],
        }
    }
}
