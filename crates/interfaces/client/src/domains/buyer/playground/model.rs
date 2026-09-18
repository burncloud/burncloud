//! Immutable catalog and value types for the buyer playground.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaygroundModel {
    pub id: &'static str,
    pub name: &'static str,
    pub input_price_per_million: f64,
    pub output_price_per_million: f64,
}

pub const MODEL_CATALOG: [PlaygroundModel; 6] = [
    PlaygroundModel {
        id: "deepseek-v3",
        name: "DeepSeek V3 (671B MoE)",
        input_price_per_million: 0.14,
        output_price_per_million: 0.28,
    },
    PlaygroundModel {
        id: "deepseek-r1",
        name: "DeepSeek R1 Reasoning",
        input_price_per_million: 0.55,
        output_price_per_million: 2.19,
    },
    PlaygroundModel {
        id: "qwen-2.5-72b-instruct",
        name: "Qwen 2.5 72B Instruct",
        input_price_per_million: 0.35,
        output_price_per_million: 0.70,
    },
    PlaygroundModel {
        id: "claude-3-5-sonnet",
        name: "Claude 3.5 Sonnet Pass-Through",
        input_price_per_million: 3.0,
        output_price_per_million: 15.0,
    },
    PlaygroundModel {
        id: "llama-3.3-70b-instruct",
        name: "Llama 3.3 70B Instruct",
        input_price_per_million: 0.60,
        output_price_per_million: 0.60,
    },
    PlaygroundModel {
        id: "glm-4-plus",
        name: "GLM-4 Plus",
        input_price_per_million: 0.70,
        output_price_per_million: 1.40,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoutingTier {
    Economy,
    Standard,
    Performance,
}

impl RoutingTier {
    pub const ALL: [Self; 3] = [Self::Economy, Self::Standard, Self::Performance];

    pub const fn api_name(self) -> &'static str {
        match self {
            Self::Economy => "economy",
            Self::Standard => "standard",
            Self::Performance => "performance",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Economy => "Economy",
            Self::Standard => "Standard",
            Self::Performance => "Performance",
        }
    }

    pub const fn ttft_ms(self) -> u32 {
        match self {
            Self::Economy => 230,
            Self::Standard => 148,
            Self::Performance => 112,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeLanguage {
    Curl,
    Python,
    Node,
}

impl CodeLanguage {
    pub const ALL: [Self; 3] = [Self::Curl, Self::Python, Self::Node];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Curl => "Curl",
            Self::Python => "Python",
            Self::Node => "Node",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InferenceStats {
    pub ttft_ms: u32,
    pub total_ms: u32,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cost_usd: f64,
}

pub const STREAMED_RESPONSE: &str = "Hardware attestation provides cryptographic verification that inference workloads run strictly inside an isolated hardware enclave (e.g. NVIDIA H100 Confidential Computing or Nitro TPM enclaves) without memory inspection or host tampering.\n\nKey architectural pillars:\n1. Root of Trust: An encrypted attestation signature is generated at the silicon level prior to weight loading.\n2. Zero-Retention Memory: Activations and KV caches are wiped immediately upon stream termination.\n3. Cryptographic Token Receipts: Every completion chunk is signed with the enclave private key, proving zero prompt logging or intermediary proxy interception.";
