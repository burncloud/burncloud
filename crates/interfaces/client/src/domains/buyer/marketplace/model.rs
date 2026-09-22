//! Immutable model catalog and value types for the buyer marketplace.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelCategory {
    All,
    General,
    Reasoning,
    Coding,
    LowLatency,
}

impl ModelCategory {
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::General,
        Self::Reasoning,
        Self::Coding,
        Self::LowLatency,
    ];

    pub const fn catalog_name(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::General => Some("General LLM"),
            Self::Reasoning => Some("Reasoning & Math"),
            Self::Coding => Some("Coding"),
            Self::LowLatency => Some("Low Latency"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelStatus {
    Healthy,
    Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoutingTier {
    Economy,
    Standard,
    Performance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Benchmark {
    pub name: &'static str,
    pub score: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarketplaceModel {
    pub id: &'static str,
    pub name: &'static str,
    pub family: &'static str,
    pub tagline: &'static str,
    pub description: &'static str,
    pub category: ModelCategory,
    pub input_price_per_million: f64,
    pub output_price_per_million: f64,
    pub context_window: &'static str,
    pub availability: f64,
    pub status: ModelStatus,
    pub supported_tiers: &'static [RoutingTier],
    pub p95_latency_ms: u32,
    pub throughput_tokens_per_sec: u32,
    pub capabilities: &'static [&'static str],
    pub benchmarks: &'static [Benchmark],
    pub recommended_for: &'static str,
}

const ECONOMY_STANDARD_PERFORMANCE: &[RoutingTier] = &[
    RoutingTier::Economy,
    RoutingTier::Standard,
    RoutingTier::Performance,
];
const STANDARD_PERFORMANCE: &[RoutingTier] = &[RoutingTier::Standard, RoutingTier::Performance];
const ECONOMY_STANDARD: &[RoutingTier] = &[RoutingTier::Economy, RoutingTier::Standard];

pub const MODEL_CATALOG: [MarketplaceModel; 6] = [
    MarketplaceModel {
        id: "deepseek-v3",
        name: "DeepSeek V3 (671B MoE)",
        family: "DeepSeek",
        tagline: "Leading open-weights frontier model with top-tier efficiency.",
        description: "High-throughput 671B Mixture-of-Experts architecture delivering GPT-4o parity at 1/10th the inference cost. Optimized for agentic execution and general intelligence.",
        category: ModelCategory::General,
        input_price_per_million: 0.14,
        output_price_per_million: 0.28,
        context_window: "128K tokens",
        availability: 99.99,
        status: ModelStatus::Healthy,
        supported_tiers: ECONOMY_STANDARD_PERFORMANCE,
        p95_latency_ms: 380,
        throughput_tokens_per_sec: 85,
        capabilities: &[
            "Tool Calling",
            "Structured JSON Output",
            "System Prompts",
            "FIM Completion",
        ],
        benchmarks: &[
            Benchmark {
                name: "MMLU-Pro",
                score: "75.9%",
            },
            Benchmark {
                name: "HumanEval",
                score: "82.6%",
            },
            Benchmark {
                name: "MATH-500",
                score: "90.2%",
            },
        ],
        recommended_for: "Enterprise production agents, high-volume classification, and conversational backends.",
    },
    MarketplaceModel {
        id: "deepseek-r1",
        name: "DeepSeek R1 Reasoning",
        family: "DeepSeek",
        tagline: "Chain-of-Thought deep reasoning for complex coding, math, and architecture.",
        description: "Incentivized reinforcement learning model exhibiting self-verification and deep multi-step deduction for hard technical questions and algorithmic design.",
        category: ModelCategory::Reasoning,
        input_price_per_million: 0.55,
        output_price_per_million: 2.19,
        context_window: "128K tokens",
        availability: 99.95,
        status: ModelStatus::Healthy,
        supported_tiers: STANDARD_PERFORMANCE,
        p95_latency_ms: 620,
        throughput_tokens_per_sec: 55,
        capabilities: &[
            "Extended Thinking",
            "Verification Loops",
            "LaTeX Math",
            "Algorithm Synthesis",
        ],
        benchmarks: &[
            Benchmark {
                name: "AIME 2024",
                score: "79.8%",
            },
            Benchmark {
                name: "MATH-500",
                score: "97.3%",
            },
            Benchmark {
                name: "Codeforces",
                score: "96.3th percentile",
            },
        ],
        recommended_for: "Complex algorithmic challenges, financial modeling, and self-correcting agents.",
    },
    MarketplaceModel {
        id: "qwen-2.5-72b-instruct",
        name: "Qwen 2.5 72B Instruct",
        family: "Qwen",
        tagline: "Versatile multilingual powerhouse with exceptional instruction following.",
        description: "Superb multilingual comprehension with state-of-the-art coding and math benchmarks. Robust reasoning with low TTFT latency.",
        category: ModelCategory::Coding,
        input_price_per_million: 0.35,
        output_price_per_million: 0.70,
        context_window: "128K tokens",
        availability: 99.98,
        status: ModelStatus::Healthy,
        supported_tiers: ECONOMY_STANDARD_PERFORMANCE,
        p95_latency_ms: 410,
        throughput_tokens_per_sec: 72,
        capabilities: &[
            "29+ Languages",
            "Long Context",
            "JSON Extraction",
            "Role Playing",
        ],
        benchmarks: &[
            Benchmark {
                name: "MMLU",
                score: "86.1%",
            },
            Benchmark {
                name: "LiveCodeBench",
                score: "42.8%",
            },
            Benchmark {
                name: "Arena-Hard",
                score: "81.2",
            },
        ],
        recommended_for: "Global multilingual applications, coding copilots, and structured data pipelines.",
    },
    MarketplaceModel {
        id: "claude-3-5-sonnet",
        name: "Claude 3.5 Sonnet Pass-Through",
        family: "Anthropic",
        tagline: "Industry benchmark for code generation and nuanced instruction following.",
        description: "Direct attestation pass-through with hardware receipt cryptographic verification and zero telemetry retention.",
        category: ModelCategory::Coding,
        input_price_per_million: 3.0,
        output_price_per_million: 15.0,
        context_window: "200K tokens",
        availability: 99.99,
        status: ModelStatus::Healthy,
        supported_tiers: STANDARD_PERFORMANCE,
        p95_latency_ms: 540,
        throughput_tokens_per_sec: 68,
        capabilities: &[
            "Artifact Rendering",
            "Vision Parsing",
            "Complex Refactoring",
            "Tool Calling",
        ],
        benchmarks: &[
            Benchmark {
                name: "SWE-bench Verified",
                score: "49.0%",
            },
            Benchmark {
                name: "TAU-bench",
                score: "69.2%",
            },
        ],
        recommended_for: "High-autonomy coding agents, UI synthesis, and mission-critical workflows.",
    },
    MarketplaceModel {
        id: "llama-3.3-70b-instruct",
        name: "Llama 3.3 70B Instruct",
        family: "Meta",
        tagline: "High-speed open standard for reliable enterprise generation.",
        description: "Meta flagship open weights model fine-tuned for dense instruction adherence, low-jitter throughput, and high concurrency.",
        category: ModelCategory::LowLatency,
        input_price_per_million: 0.60,
        output_price_per_million: 0.60,
        context_window: "128K tokens",
        availability: 99.97,
        status: ModelStatus::Healthy,
        supported_tiers: ECONOMY_STANDARD,
        p95_latency_ms: 290,
        throughput_tokens_per_sec: 96,
        capabilities: &["Fast Completion", "Function Calling", "RAG Embed Support"],
        benchmarks: &[
            Benchmark {
                name: "MMLU",
                score: "88.6%",
            },
            Benchmark {
                name: "GSM8K",
                score: "95.0%",
            },
        ],
        recommended_for: "Cost-sensitive real-time chat, summarization, and batch RAG pipelines.",
    },
    MarketplaceModel {
        id: "glm-4-plus",
        name: "GLM-4 Plus",
        family: "Zhipu",
        tagline: "Massive context comprehension with Chinese language mastery.",
        description: "State-of-the-art Chinese comprehension and bilingual reasoning with 1M tokens extended context handling capabilities.",
        category: ModelCategory::General,
        input_price_per_million: 0.70,
        output_price_per_million: 1.40,
        context_window: "1M tokens",
        availability: 99.91,
        status: ModelStatus::Degraded,
        supported_tiers: STANDARD_PERFORMANCE,
        p95_latency_ms: 780,
        throughput_tokens_per_sec: 48,
        capabilities: &["1M Long Document QA", "Bilingual Translation", "Workflow Logic"],
        benchmarks: &[Benchmark {
            name: "LongBench-Chat",
            score: "89.4%",
        }],
        recommended_for: "Full-book processing, legal contract analysis, and bilingual knowledge search.",
    },
];
