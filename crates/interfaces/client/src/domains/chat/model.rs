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

/// Chat 首页菜单展示的五个在线模型。
pub(super) fn catalog() -> [ChatModel; 5] {
    let [first, second, third, fourth, fifth, _] = MODEL_CATALOG;
    [first, second, third, fourth, fifth].map(|model| ChatModel { model })
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
    pub(super) thinking_open: bool,
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
    "BurnCloud 通过 **AMD SEV-SNP** 与 **AWS Nitro Enclaves** 的硬件级根信任签名机制，保障本轮推理不会留下可恢复的数据残留。",
    "\n\n1. **硬件级内存加密**：请求上下文只在独立密钥保护的 Enclave 内存中处理。",
    "\n2. **远程认证收据**：客户端可以校验 TPM 2.0 测量值和节点签名，确认运行环境没有被替换。",
    "\n3. **内存即时零化**：TLS 1.3 会话结束后，推理缓冲区会被覆写清零。",
    "\n\n这是一条由 Chat v1 提供的确定性本地模拟回复。",
];

/// 返回参考站首页首次加载时展示的三条本地会话。
pub(super) fn sample_conversations() -> Vec<ChatConversation> {
    vec![
        ChatConversation {
            id: "conversation-1".to_string(),
            title: "DeepSeek R1 推理与硬件证明".to_string(),
            model_id: "deepseek-r1".to_string(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: "请说明 BurnCloud 的硬件 Enclave 机密计算是如何证明零数据残留的？"
                        .to_string(),
                    thinking: None,
                    thinking_open: false,
                    streaming: false,
                    copied: false,
                    metrics: None,
                },
                ChatMessage {
                    role: MessageRole::Assistant,
                    content: r#"BurnCloud 采用 **AMD SEV-SNP** 与 **AWS Nitro Enclaves** 硬件级根信任签名机制保障零数据残留：

1. **硬件级内存加密**：虚拟机内部内存均由处理器内置的安全处理器（ASP）使用独立密钥硬件级加密，宿主机操作系统和物理机所有者均无法窥探。
2. **密码学硬件认证（Remote Attestation）**：每次推理请求返回的响应头均包含来自 TPM 2.0 芯片的私钥签名哈希，可由客户端通过厂商根证书公钥直接验证。
3. **内存即时零化（Memory Zeroization）**：TLS 1.3 会话结束时，Enclave 内部微内核会自动执行内存零化覆写，确保上下文不持久化至磁盘。

```json
{
  "burncloud_receipt": {
    "enclave_type": "AMD_SEV_SNP_GEN4",
    "pcr0_measurement": "a8f94e2b0c11d94f...",
    "zeroization_policy": "STRICT_EPHEMERAL",
    "verified_at": "2026-10-20T05:50:00Z"
  }
}
```"#
                        .to_string(),
                    thinking: Some("分析用户对去中心化算力隐私与零残留机密性的关注点：需从硬件根信任（AMD SEV-SNP）、PCR度量证明、以及传输断开时的内存覆写技术机制进行详细阐述。".to_string()),
                    thinking_open: false,
                    streaming: false,
                    copied: false,
                    metrics: Some(InferenceMetrics {
                        tokens_per_second: 128.4,
                        ttft_ms: 41,
                        node: "AMD SEV-SNP #H100-08",
                        cost_usd: 0.00014,
                        prompt_tokens: 42,
                        completion_tokens: 184,
                    }),
                },
            ],
        },
        ChatConversation {
            id: "conversation-2".to_string(),
            title: "Python 异步流式客户端".to_string(),
            model_id: "deepseek-v3".to_string(),
            messages: Vec::new(),
        },
        ChatConversation {
            id: "conversation-3".to_string(),
            title: "Solana 质押与 SLA 罚没".to_string(),
            model_id: "llama-3.3-70b-instruct".to_string(),
            messages: Vec::new(),
        },
    ]
}
