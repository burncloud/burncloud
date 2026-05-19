# 供应商 Tier 分级规范

> **版本**: v1.0
> **最后更新**: 2026-05-19

---

## 一、Tier 分级定义

### UpstreamTier 枚举

```rust
/// 供应商层级（Issue #213）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum UpstreamTier {
    /// 一手货源（官方直连）
    Official = 0,
    /// 信誉中转
    ReputableRelay = 1,
    /// 风险提示（允许接入，但显示警告）
    Banned = 2,
}
```

### Tier 分级标准

| Tier | 名称 | 说明 | 更新权限 |
|------|------|------|----------|
| **Tier 0** | Official | 一手货源，官方直连 | 仅 BurnCloud 官方团队通过 PR |
| **Tier 1** | ReputableRelay | 信誉中转，聚合平台 | 仅 BurnCloud 官方团队通过 PR |
| **Tier 2** | Banned | 风险提示，未知平台 | 系统默认，无需配置 |

---

## 二、Tier 0 一手货源（官方直连）

### 认定条件

1. **官方运营**：由模型开发商或云厂商直接运营
2. **合规保障**：提供企业级 SLA、对公发票、数据合规
3. **稳定可靠**：长期运营历史，无重大事故
4. **直接授权**：拥有模型官方授权或自研模型

### 国际大厂

| 平台 | 官网 | API 文档 | 特点 |
|------|------|----------|------|
| **OpenAI** | https://openai.com | https://platform.openai.com/docs | GPT-4o, GPT-4 Turbo, o1, o3 系列 |
| **Anthropic** | https://anthropic.com | https://docs.anthropic.com | Claude 3.5/4 系列，安全对齐领先 |
| **Google AI (Gemini)** | https://ai.google.dev | https://ai.google.dev/docs | Gemini 2.0/1.5 系列，多模态强 |
| **Google Vertex AI** | https://cloud.google.com/vertex-ai | https://cloud.google.com/vertex-ai/docs | 企业级，支持第三方模型 |
| **AWS Bedrock** | https://aws.amazon.com/bedrock | https://docs.aws.amazon.com/bedrock | 多模型聚合，企业级基础设施 |
| **Azure OpenAI** | https://azure.microsoft.com/openai | https://learn.microsoft.com/azure/ai-services | OpenAI 模型 + Azure 企业合规 |
| **Meta (Llama)** | https://llama.meta.com | https://huggingface.co/meta-llama | 开源模型，需自行部署或通过第三方 |
| **Mistral AI** | https://mistral.ai | https://docs.mistral.ai | Mistral Large/Medium/Small，欧洲领先 |
| **Cohere** | https://cohere.com | https://docs.cohere.com | 企业 NLP，Command/Rerank 系列 |
| **AI21 Labs** | https://ai21.com | https://docs.ai21.com | Jamba 系列，长文本处理 |
| **xAI (Grok)** | https://x.ai | https://docs.x.ai | Grok-2/3，X 平台集成 |
| **Perplexity AI** | https://perplexity.ai | https://docs.perplexity.ai | 搜索增强推理，Sonar 系列 |
| **Replicate** | https://replicate.com | https://replicate.com/docs | 模型托管平台，开源模型即服务 |

### 国产大厂

| 平台 | 公司 | 官网 | API 文档 | 特点 |
|------|------|------|----------|------|
| **阿里云百炼** | 阿里巴巴 | https://bailian.console.aliyun.com | https://help.aliyun.com/bailian | 通义千问 Qwen 系列，企业生态完善 |
| **百度智能云** | 百度 | https://cloud.baidu.com | https://cloud.baidu.com/doc/WENXINWORKSHOP | 文心一言 ERNIE 系列，中文理解强 |
| **腾讯混元** | 腾讯 | https://cloud.tencent.com/product/hunyuan | https://cloud.tencent.com/document/product/1729 | 混元系列，微信生态集成 |
| **字节豆包** | 字节跳动 | https://www.volcengine.com/product/doubao | https://www.volcengine.com/docs/82379 | 豆包系列，日调用量超千亿 tokens |
| **华为云盘古** | 华为 | https://www.huaweicloud.com/product/pangu.html | https://support.huaweicloud.com/pangu | 盘古系列，政企市场强 |
| **商汤日日新** | 商汤科技 | https://www.sensetime.com | https://platform.sensenova.cn | 日日新系列，多模态视觉强 |

### 国产 AI 六小虎（创业公司）

| 平台 | 公司 | 官网 | API 文档 | 特点 |
|------|------|------|----------|------|
| **DeepSeek** | 深度求索 | https://deepseek.com | https://platform.deepseek.com | DeepSeek-V3/R1，性价比之王，开源领先 |
| **Moonshot (Kimi)** | 月之暗面 | https://moonshot.ai | https://platform.moonshot.cn | Kimi K2.5，长文本处理领先 |
| **智谱 AI (ZhipuAI)** | 智谱清言 | https://www.zhipuai.cn | https://open.bigmodel.cn | GLM-5 系列，清华系，开源生态 |
| **MiniMax** | MiniMax | https://www.minimaxi.com | https://api.minimax.chat | M2.5 系列，语音多模态强 |
| **百川智能** | 百川 | https://www.baichuan-ai.com | https://platform.baichuan-ai.com | Baichuan 4 系列，中文优化 |
| **阶跃星辰** | Step | https://www.stepfun.com | https://platform.stepfun.com | Step-2 万亿参数 MoE，跃问产品 |
| **零一万物** | Yi | https://www.lingyiwanwu.com | https://platform.lingyiwanwu.com | Yi-Lightning 系列，李开复博士创立 |
| **昆仑万维天工** | 昆仑万维 | https://www.tiangong.cn | https://model.tiangong.cn | 天工 4.0，搜索增强 |

---

## 三、Tier 1 信誉中转（聚合平台）

### 认定条件

1. **信誉良好**：社区口碑好，运营透明
2. **规模较大**：用户量大，有真实业务支撑
3. **多模型聚合**：接入多家 Tier 0 供应商
4. **企业服务**：提供企业管理功能

### 国际聚合平台

| 平台 | 官网 | 特点 | 模型数量 |
|------|------|------|----------|
| **OpenRouter** | https://openrouter.ai | 海外最大聚合商，AI 极客首选，价格透明 | 300+ 模型 |
| **Together AI** | https://together.ai | 开源模型推理领先，性价比高 | 100+ 模型 |
| **Groq** | https://groq.com | LPU 推理引擎，极速响应 | 20+ 模型 |
| **Fireworks AI** | https://fireworks.ai | 低延迟推理，开源模型优化 | 50+ 模型 |
| **Anyscale** | https://anyscale.com | Ray 框架团队，企业级推理 | 30+ 模型 |
| **NVIDIA NIM** | https://build.nvidia.com | GPU 厂商官方，企业级部署 | 40+ 模型 |
| **Hugging Face Inference** | https://huggingface.co/inference | 开源社区官方，模型最全 | 1000+ 模型 |
| **Fal.ai** | https://fal.ai | 图像/视频生成专用 | 多模态 |
| **RunPod** | https://runpod.io | GPU 租赁 + 模型部署 | 自定义 |

### 国内聚合平台

| 平台 | 官网 | 特点 | 模型数量 |
|------|------|------|----------|
| **硅基流动 (SiliconFlow)** | https://siliconflow.cn | 国产聚合领先，价格优势 | 100+ 模型 |
| **n1n.ai** | https://n1n.ai | 全栈聚合，三协议兼容 | 480+ 模型 |
| **非线智能 API** | https://feixian.ai | 企业管理完善，99.99% SLA | 480+ 模型 |
| **4S API** | https://4s.ai | 商业落地首选，对公发票 | 300+ 模型 |
| **PoloAPI** | https://poloapi.com | 开发者友好 | 200+ 模型 |
| **weelinking** | https://weelinking.com | 企业级服务 | 200+ 模型 |
| **302.AI** | https://302.ai | 国内老牌中转 | 100+ 模型 |
| **CloseAI** | https://closeai-asia.com | OpenAI 亚洲优化 | 50+ 模型 |

### 云厂商 AI Gateway

| 平台 | 官网 | 特点 |
|------|------|------|
| **Cloudflare AI Gateway** | https://developers.cloudflare.com/ai-gateway | 边缘网络，全球节点，多模型聚合 |
| **Portkey** | https://portkey.ai | AI 网关，可观测性强，企业级 |
| **LiteLLM** | https://litellm.ai | 开源网关，自部署首选 |

---

## 四、Tier 2 风险提示（未知/小众平台）

> 默认归类，允许接入但需显示风险警告

### 风险类型

- **NewApi** 及其衍生项目
- 个人/小团队运营的中转站
- 无官方背书的第三方聚合
- 价格异常低的平台（可能跑路风险）

### 客户选择权原则

Tier 2 渠道**不禁止接入**，但必须**明确告知客户风险**，让客户自行决定。

---

## 五、Tier 配置更新机制

### 更新权限

| Tier | 更新权限 | 理由 |
|------|----------|------|
| **Tier 0** | 仅 BurnCloud 官方团队通过 PR | 涉及官方信誉背书 |
| **Tier 1** | 仅 BurnCloud 官方团队通过 PR | 涉及信誉认证 |
| **Tier 2** | 系统默认（未知平台） | 无需配置 |

### 禁止操作

| 操作 | 是否允许 | 理由 |
|------|----------|------|
| 用户通过 API 添加 Tier 0 | ❌ 禁止 | 涉及官方信誉背书 |
| 用户通过 API 添加 Tier 1 | ❌ 禁止 | 涉及信誉认证 |
| 用户通过配置文件修改 Tier | ❌ 禁止 | 防止绕过官方认定 |
| 官方通过 PR 更新 Tier 0/1 | ✅ 允许 | 经过审核流程 |

### 新增 Tier 0/1 流程

```
1. 提交 GitHub Issue 申请加入 Tier 0/1
   - 说明平台背景
   - 提供官方文档链接
   - 说明为何符合 Tier 0/1 标准

2. BurnCloud 官方团队审核
   - 验证平台官方性质
   - 评估信誉和稳定性
   - 决定是否批准

3. 批准后提交 PR 更新代码
   - 修改 UpstreamTier::from_channel_type()
   - 更新 PROVIDERS.md 文档
   - 通过 CI 测试后合并
```

---

## 六、代码实现

### ChannelType 到 Tier 映射

```rust
impl UpstreamTier {
    pub fn from_channel_type(channel_type: ChannelType) -> Self {
        match channel_type {
            // Tier 0: 一手货源（官方直连）
            ChannelType::OpenAI 
            | ChannelType::Anthropic 
            | ChannelType::AWSBedrock 
            | ChannelType::AzureOpenAI 
            | ChannelType::GoogleAI 
            | ChannelType::VertexAI 
            | ChannelType::AliyunBailian
            | ChannelType::DeepSeek
            | ChannelType::Moonshot
            | ChannelType::ZhipuAI
            => UpstreamTier::Official,
            
            // Tier 1: 信誉中转
            ChannelType::OpenRouter 
            | ChannelType::TogetherAI 
            | ChannelType::Groq 
            => UpstreamTier::ReputableRelay,
            
            // Tier 2: 风险提示（未知平台自动归类）
            _ => UpstreamTier::Banned,
        }
    }
}
```

### Channel 结构体扩展

```rust
pub struct Channel {
    // ... 现有字段
    /// 供应商层级（Issue #213）
    pub upstream_tier: i32, // 0=官方, 1=信誉中转, 2=风险提示
}
```

---

## 七、参考链接

- [LLM 价格对比](https://llmprice.cn)
- [LLM Reference Providers](https://www.llmreference.com/providers)
- [腾讯云：大模型 API 中转站选型指南](https://cloud.tencent.com/developer/article/2667472)
