#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeploymentStatus {
    Healthy,
    Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Deployment {
    pub model: &'static str,
    pub node: &'static str,
    pub parallelism: &'static str,
    pub throughput: &'static str,
    pub contribution: &'static str,
    pub status: DeploymentStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupplierDeploymentsModel {
    pub deployments: [Deployment; 4],
}

pub const REFERENCE_DEPLOYMENTS: SupplierDeploymentsModel = SupplierDeploymentsModel {
    deployments: [
        Deployment {
            model: "DeepSeek V3 (671B MoE)",
            node: "SJC-Pod-01-Rack4 (8x H100)",
            parallelism: "TP=8 / FP8 FlashInfer",
            throughput: "85.2 tokens/s",
            contribution: "99.8 / 100",
            status: DeploymentStatus::Healthy,
        },
        Deployment {
            model: "DeepSeek R1 Reasoning",
            node: "SJC-Pod-01-Rack5 (8x H100)",
            parallelism: "TP=8 / DeepSeek-VLLM",
            throughput: "56.4 tokens/s",
            contribution: "99.4 / 100",
            status: DeploymentStatus::Healthy,
        },
        Deployment {
            model: "Qwen 2.5 72B Instruct",
            node: "FRA-DC2-Compute-08 (8x A100)",
            parallelism: "TP=8 / vLLM v0.6.2",
            throughput: "72.0 tokens/s",
            contribution: "98.9 / 100",
            status: DeploymentStatus::Healthy,
        },
        Deployment {
            model: "Llama 3.3 70B Quantized",
            node: "HKG-Edge-RTX-Pool (4x 4090)",
            parallelism: "TP=4 / AWQ INT4",
            throughput: "42.1 tokens/s",
            contribution: "92.1 / 100",
            status: DeploymentStatus::Degraded,
        },
    ],
};

impl SupplierDeploymentsModel {
    pub const fn reference() -> &'static Self {
        &REFERENCE_DEPLOYMENTS
    }
}
