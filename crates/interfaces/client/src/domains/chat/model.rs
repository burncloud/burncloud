//! Chat 首页使用的展示模型和本地示例数据。

use crate::domains::buyer::marketplace::model::{MarketplaceModel, ModelStatus, MODEL_CATALOG};

/// Chat 首页可选择的模型展示适配。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ChatModel {
    pub(super) model: MarketplaceModel,
}

impl ChatModel {
    pub(super) fn id(self) -> &'static str {
        self.model.id
    }

    pub(super) fn is_online(self) -> bool {
        self.model.status == ModelStatus::Healthy
    }
}

pub(super) fn catalog() -> [ChatModel; 6] {
    MODEL_CATALOG.map(|model| ChatModel { model })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code, reason = "保留 System 角色以对齐消息协议，v1 不直接展示")]
pub(super) enum MessageRole {
    User,
    Assistant,
    System,
}

/// 助手回复的本地展示指标。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct InferenceMetrics {
    pub(super) tokens_per_second: f32,
    pub(super) ttft_ms: u32,
    pub(super) node: &'static str,
    pub(super) cost_usd: f64,
    pub(super) prompt_tokens: u32,
    pub(super) completion_tokens: u32,
}

/// 一条 Chat 消息及其展示状态。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ChatMessage {
    pub(super) role: MessageRole,
    pub(super) content: String,
    pub(super) thinking: Option<String>,
    pub(super) streaming: bool,
    pub(super) copied: bool,
    pub(super) metrics: Option<InferenceMetrics>,
}

/// Chat 首页的一条会话记录。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ChatConversation {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) model_id: String,
    pub(super) messages: Vec<ChatMessage>,
}

/// 模拟回复的固定分段，保证 v1 不引入远端推理依赖。
pub(super) const SIMULATED_REPLY_SEGMENTS: [&str; 5] = [
    "## 结论\n\nBurnCloud 将模型请求路由到具备硬件证明的推理节点，并把每次回复的验证结果保留在本地展示状态中。",
    "\n\n- 首先，网关确认模型、温度和上下文配置。",
    "\n- 其次，节点在生成前完成隔离环境证明。",
    "\n\n```rust\nfn verify_attestation() -> bool {\n    true\n}\n```",
    "\n\n因此，用户可以同时看到可读的回复、推理指标和安全说明；这段回复是 Chat v1 的确定性模拟数据。",
];

pub(super) fn sample_conversations() -> Vec<ChatConversation> {
    vec![
        ChatConversation {
            id: "conversation-1".to_string(),
            title: "DeepSeek R1 推理与硬件证明".to_string(),
            model_id: "deepseek-r1".to_string(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: "硬件证明如何保证多租户 LLM 推理的安全？".to_string(),
                    thinking: None,
                    streaming: false,
                    copied: false,
                    metrics: None,
                },
                ChatMessage {
                    role: MessageRole::Assistant,
                    content: "## 核心机制\n\n硬件证明把模型权重加载、请求处理和密钥使用限制在可验证的隔离环境内。平台可以在返回 Token 前验证节点身份、运行时哈希和策略签名。\n\n这使得路由层可以提供可追溯的执行结果，而不需要读取租户的提示词或中间激活值。".to_string(),
                    thinking: Some("先确认威胁模型，再分别说明节点身份、内存隔离和结果签名之间的关系。".to_string()),
                    streaming: false,
                    copied: false,
                    metrics: Some(InferenceMetrics {
                        tokens_per_second: 55.0,
                        ttft_ms: 148,
                        node: "US-West · H100",
                        cost_usd: 0.000426,
                        prompt_tokens: 42,
                        completion_tokens: 184,
                    }),
                },
            ],
        },
        ChatConversation {
            id: "conversation-2".to_string(),
            title: "模型路由与 SLA 保障".to_string(),
            model_id: "deepseek-v3".to_string(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: "如何为高并发应用选择模型路由策略？".to_string(),
                    thinking: None,
                    streaming: false,
                    copied: false,
                    metrics: None,
                },
                ChatMessage {
                    role: MessageRole::Assistant,
                    content: "可以先按延迟、吞吐量和成本设置路由优先级，再使用节点健康度和容量进行实时回退。对高并发场景，建议把低延迟模型作为默认路径，把深度推理模型保留给复杂请求。".to_string(),
                    thinking: Some("比较吞吐量、P95 延迟和路由回退的约束。".to_string()),
                    streaming: false,
                    copied: false,
                    metrics: Some(InferenceMetrics {
                        tokens_per_second: 85.0,
                        ttft_ms: 112,
                        node: "EU-Central · H100",
                        cost_usd: 0.000057,
                        prompt_tokens: 31,
                        completion_tokens: 97,
                    }),
                },
            ],
        },
        ChatConversation {
            id: "conversation-3".to_string(),
            title: "零留存推理说明".to_string(),
            model_id: "qwen-2.5-72b-instruct".to_string(),
            messages: Vec::new(),
        },
    ]
}
