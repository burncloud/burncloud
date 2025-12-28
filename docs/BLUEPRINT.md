# BurnCloud Product Roadmap & Feature Specification

> **Last Updated:** 2025-12-28
> **Status:** Active Planning
> **Target Audience:** Operational Development Experts (懂运营的开发专家)
> **Philosophy:** The Cockpit (Profit & Control), Business Engine, Developer Ergonomics.

This document outlines the functional planning for the BurnCloud Client, designed as a **Business Engine** for developers who operate AI services.

---

## 1. 仪表盘 (Dashboard)
**Codename:** *Zen Command*
**Module:** `client-dashboard`

### 核心理念
**Profit Watch (利润监控)**。即使在睡觉时，也要让运营者一眼看到系统在赚钱。从“系统健康”转向“商业健康”。

### 深度规划
- **利润核心 (Profit Core)**:
    - **三大指标**: 屏幕中央展示巨大的 **Net Margin (净利润)**，辅以 **Revenue (营收)** 和 **Cost (成本)**。
    - **趋势线**: 实时 TPS (Tokens Per Second) 曲线，直观感受业务心跳。
- **系统脉搏 (System Pulse)**:
    - **极简状态**: 一个简单的呼吸灯。Green = Money Flowing. Red = Action Needed.
    - **异常告警**: 仅在利润受损时（如渠道故障、高错误率）弹出醒目的 Action Item。
- **资源概览**:
    - 活跃的下游客户数量 (Active Customers)。
    - 当前可用的上游算力池深度 (Pool Depth)。

---

## 2. 模型网络 (Model Network)
**Codename:** *The Nexus*
**Module:** `client-models` & `service-models`

### 核心理念
全球大脑聚合 (Global Brain Aggregation)。无论模型运行在本地显卡、AWS 的机房还是 Google 的 TPU 上，在这里它们都是平等的“专家”。BurnCloud 是连接它们的统一神经中枢。

### 深度规划
- **云端集成 (Cloud Integrations)**:
    - **AWS Bedrock**: 深度集成 SigV4 签名。支持选择区域 (Region, e.g., us-east-1) 并自动列出该区域可用的 Claude/Titan 模型。
    - **Azure OpenAI**: 支持配置 Deployment ID 和 API Version。自动映射 Azure 特有的 Endpoint 结构。
    - **Google Vertex AI / Gemini**: 支持 Service Account JSON 导入，自动处理 OAuth2 Token 刷新。
    - **Native API**: 标准支持 OpenAI, Anthropic, Groq 等直接提供的 API。
    - **Model Auto-Discovery**: 模型添加应自动识别目标渠道具备哪些模型，或在用户请求时动态探查 (Probe-on-Demand)，严禁要求用户手动填写模型名称。
- **本地仓库 (Local Repository)**:
    - **GGUF Manager**: 扫描并管理本地 `models/` 目录下的量化模型。
    - **One-Click Serve**: 针对 GGUF 文件一键启动本地推理服务 (llama-server)，并自动注册到路由表中。
- **统一接口 (Unified Interface)**:
    - **Model Standardization**: 无论上游是 Azure 还是本地，在列表中统一展示为标准的模型卡片 (Context Window, Max Tokens)。
    - **Latency Test**: 一键测试所有已配置模型的连通性和延迟 (Ping)。

---

## 3. 算力互联 (BurnCloud Connect)
**Codename:** *Fabric*
**Module:** `client-connect`

### 核心理念
**“算力接入点” (The Compute Access Point)**。从去中心化市场转型为**托管算力矿池 (Managed Compute Pool)**。我们构建基础设施（隧道与风控），由合规运营公司负责资金与服务管理。

### 深度规划
- **供应端: 算力贡献 (Supply: Contribution)**
    - **无感接入**: 用户仅需填入 AWS/Azure 凭证，凭证在本地加密存储。
    - **隐形隧道**: 集成 P2P/QUIC 协议，无需公网 IP 即可将本地资源接入全球网络。
    - **混合动力模式 (Hybrid Engine)**:
        - **自营优先**: 优先调度用户自己填入的本地 API Key。
        - **溢出采购 (Overflow Sourcing)**: 当自营 Key 配额耗尽或并发过高时，自动无缝切换至 BurnCloud 官方托管集群进行补货，赚取中间差价。
- **采购端: 资源获取 (Demand: Sourcing)**
    - **托管集群 (Managed Clusters)**: 用户直接连接到官方认证的“高优算力集群” (如 "SkyNet Prime")，享受企业级 SLA。
    - **透明消费**: 无需购买 Key，直接购买“额度包”。Router 自动在后台撮合请求到最优供应节点。
- **风控雷达 (Risk Radar Integration)**
    - **双向保护**: 既防止买家欺诈（虚假算力），也防止卖家被滥用（防炼丹、防非法内容）。
    - **蜜罐探测**: 系统自动发送隐形测试请求，识别虚假节点或低质量模型。
    - **时序指纹**: 分析 TTFT (首字延迟) 和吐字速率，确保节点真实性。
    - **惩罚机制**: 对作弊节点执行“降权”或“断流”，保障网络纯净度。

---

## 4. 访问凭证 (Access Credentials)
**Codename:** *Keymaster*
**Module:** `client-access`

### 核心理念
严密的守门人。Fail-Safe Binding（故障安全绑定）。

### 深度规划
- **API Key 生命周期管理**:
    - 创建、重置、吊销 API Key (`sk-burn...`)。
    - **自动过期**: 支持设置 Key 的有效期（如“1小时后过期”）。
- **权限与配额 (Scopes & Quotas)**:
    - **精细化权限**: 勾选该 Key 可访问的模型组 (e.g., "Only GPT-4", "Only Local Models")。
    - **预算限制**: 设置该 Key 的每日消费上限 ($ 或 Token 数)。
- **应用绑定**:
    - 强制要求 Key 绑定特定的 Application ID 或 IP 白名单，防止泄露滥用。

---

## 5. 演练场 (Playground)
**Codename:** *Arena*
**Module:** `client-playground`

### 核心理念
真实的测试环境，所见即所得。全链路点火验证区。

### 深度规划
- **多模态对话流 (Multimodal Stream)**:
    - 支持文本流式输出 (Streaming)。
    - 支持图片上传与解析 (Vision Models)。
    - 渲染 Markdown、LaTeX 公式、代码高亮。
- **参数实验室 (Hyperparameter Lab)**:
    - 侧边栏实时调整 Temperature, Top-P, Presence Penalty。
    - 实时查看调整对输出结果的影响。
- **竞技场模式 (Arena Mode)**:
    - **AB Test**: 左右分屏，同时向两个不同模型发送相同提示词，对比响应速度和质量。
    - **Raw JSON View**: 查看原始的 Request/Response JSON，便于开发者调试协议适配。

---

## 6. 风控雷达 (Risk Radar)
**Codename:** *Shield*
**Module:** `client-radar` & `client-monitor`

### 核心理念
**双向信任引擎 (Bidirectional Trust Engine)**。不仅是防火墙，更是维护算力互联生态秩序的维和部队。它既保护买家免受虚假算力欺诈，也保护卖家（供应商）免受恶意请求的侵害。

### 深度规划
- **买家侧防御 (Consumer Protection)**:
    - **真伪验证**: 结合蜜罐探测与时序指纹，实时拦截“货不对板”（如用 Llama 冒充 Claude）的虚假节点。
    - **隐私漂白 (Privacy Bleaching)**: 在请求发出前，自动识别并脱敏 PII (个人敏感信息)，确保数据离境安全。
    - **防二次转发**: 检测响应水印，防止请求被中间人二次转卖。
- **卖家侧防御 (Supplier Protection)**:
    - **炼丹阻断 (Anti-Training)**: 识别并拦截高并发、大数据量的训练类请求，防止账号因滥用被 AWS 封禁。
    - **合规防火墙**: 本地预审入站 Prompt，自动拒接色情、暴力、诈骗等高风险内容，确保账号持有者的法律安全。
    - **成本熔断**: 强制执行每小时/每日的成本硬红线，防止被意外刷爆信用卡。
- **可视化情报 (Visual Intelligence)**:
    - **信任等级 (Trust Rating)**: 实时展示当前连接节点的信誉分与蜜罐通过率。
    - **双向威胁流**: 清晰区分“入站拦截”（保护卖家）与“出站清洗”（保护买家）的安全事件。
    - **IP Compliance & MSP Governance (Anti-Ban Protocol)**:
        - **MSP Proxy Pool**: MSP 运营者可在后台维护合规的静态 IP / 代理池 (Server IPs)。
        - **Smart Allocation**: 系统根据添加的凭证类型 (Account Type) 和区域 (Region)，自动从池中分配一个专属 IP。
        - **Isolation Policy**: 严格执行 "One Key, One IP" (一号一网) 原则，杜绝因 IP 复用导致的关联封号。
        - **Pre-flight Check**: 连接建立前检测本地 IP 风险；若高风险 (Geo-Mismatch/DataCenter)，自动切换至 MSP 分配的合规隧道，确保流量特征“本地化”且“干净”。

---

## 7. 日志审查 (Log Review)
**Codename:** *Blackbox*
**Module:** `client-log`

### 核心理念
**全链路透明化 (End-to-End Transparency)**。日志不再是杂乱的文本，而是精准定位故障的“手术刀”。通过区分“入站”与“出站”两个关键跳 (Hops)，实现秒级定责与排障。

### 深度规划
- **双跳日志策略 (Two-Hop Logging)**:
    - **接入日志 (Inbound / Access Logs)**: 记录买家请求到 BurnCloud 客户端的记录。重点审计：鉴权状态 (401)、余额消耗、连接耗时 (Internal Latency)。解决“买家连不上”的问题。
    - **上游日志 (Outbound / Upstream Logs)**: 记录 BurnCloud 代理请求到 AWS/Azure 的记录。重点记录：供应商 Request ID、上游状态码 (502/429)、真实成本。解决“账号是否挂了”的问题。
- **全链路追踪 (Trace ID Integration)**:
    - 为每个请求分配唯一的 `Trace ID`。管理员点击任意一条接入日志，即可瞬间展开其对应的上游请求详情，将内、外网络交互完美串联。
- **自诊断定责**:
    - **墙内/墙外判定**: 系统自动标记错误发生的物理位置——是本地配置/网络问题 (Inbound)，还是云服务商 API 报错 (Outbound)。
    - **报错智能归类**: 自动聚合相似错误，例如：“检测到 5 次 AWS Bedrock 欠费报错，已自动暂停该账号”。
- **可视化排障面板**:
    - 提供瀑布流视图，直观展示请求在网关处理、隧道传输、上游响应三个阶段的时间分布。

---

## 8. 用户管理 (User Management)
**Codename:** *Organization*
**Module:** `client-users` & `client-register`

### 核心理念
**虚拟运营商控制台 (Virtual Operator Console)**。不仅仅是管理内部员工，更是赋能用户成为独立的 AI 算力分销商 (Reseller)。

### 深度规划
- **私有化运营 (Self-Hosted Operation)**:
    - **多租户体系**: 支持创建完全独立的下游租户 (Tenants)，每个租户拥有独立的 Quota 和 API Key。
    - **自定义费率**: 运营者可自定义对下游用户的计费倍率 (e.g., GPT-4 原价 x 1.2)。
- **人员花名册 (Roster)**:
    - 列表展示系统内所有用户（员工或下游客户）。
    - 状态指示: Online, Offline, Banned, Over-Quota。
- **角色分配 (Role Assignment)**:
    - **Owner**: 拥有最高权限，控制上游渠道和费率。
    - **Reseller**: 代理商，可以发展自己的下级用户。
    - **End-User**: 仅拥有 Chat/API 调用权限。
- **邀请与注册**:
    - 生成带有时效性的注册邀请码。
    - 支持开启/关闭公开注册入口。

---

## 9. 财务中心 (Finance Center)
**Codename:** *Ledger*
**Module:** `client-finance`

### 核心理念
每一分算力都有据可查。

### 深度规划
- **消耗大盘 (Consumption Dashboard)**:
    - 堆叠柱状图展示每日/每月的 Token 消耗，按模型/渠道区分颜色。
    - 预估当月账单金额。
- **渠道成本分析 (Channel Cost Analysis)**:
    - 饼图展示哪个上游渠道 (OpenAI, AWS, Azure) 花费最多。
    - 计算每个渠道的平均 Token 单价。
- **充值与计费 (Billing)**:
    - 如果是商业化部署，这里集成 Stripe/WeChat Pay 接口。
    - 内部部署则用于设置部门预算配额。

---

## 10. 系统设置 (System Settings)
**Codename:** *Control Panel*
**Module:** `client-settings`

### 核心理念
基础设施的控制台。Fail-Safe。

### 深度规划
- **路由策略 (Routing Strategies)**:
    - 配置全局路由算法: 轮询 (Round Robin), 价格优先 (Price Lowest), 延迟优先 (Latency Lowest)。
- **网络配置**:
    - 绑定端口 (Port), 开启/关闭 HTTPS, 配置 CORS 域名。
    - 设置上游代理 (Proxy) 以连通国际网络。
- **关于与更新 (About & Update)**:
    - 集成 `auto-update` crate，检查 GitHub Release 更新。
    - 展示当前版本 Hash, 构建时间。

---

## 11. 智能路由内核 (Intelligent Router Core)
**Codename:** *Flow*
**Module:** `router` & `router-aws`

### 核心理念
**“看不见的手” (The Invisible Hand)**。这是系统的隐形引擎，负责将用户的请求以毫秒级的速度精准投递到最优的算力节点。

### 深度规划
- **透传原则 (Absolute Passthrough)**:
    - **Don't Touch Body**: 除非必要（如 AWS SigV4 签名需要读取 Body 计算 Hash），否则 Router 绝不解析、修改 Request/Response Body。这保证了 **Zero-Latency Overhead**（零延迟开销）和 **100% Streaming Compatibility**（流式兼容）。
- **智能调度策略 (Smart Routing Strategies)**:
    - **Lowest Latency (竞速模式)**: 同时向多个渠道发起 Ping 或预检，优先路由到响应最快的节点。
    - **Lowest Price (经济模式)**: 在满足 SLA 的前提下，优先选择 Token 单价最低的供应源（例如优先用 Azure Spot 实例或便宜的第三方渠道）。
    - **Adaptive Hybrid Routing (自适应混合路由)**:
        - **Global Configuration**: 用户可在系统设置中全局指定默认策略：`Adaptive` (默认), `Always Sticky` (一致性优先), `Always Speed` (速度优先)。
        - **Smart Decision (Adaptive模式)**: Router 自动分析请求特征。
            - **Short Context (< 4k)**: 走 **竞速/比价模式**，追求极致响应速度。
            - **Long Context (> 4k)**: 自动切换为 **Sticky Mode**，锁定节点以复用 KV Cache，降低首字延迟 (TTFT) 和成本。
        - **User Override**: 允许通过 HTTP Header (`X-Burn-Strategy`) 覆盖全局设置。
    - **Weighted Round Robin (加权轮询)**: 根据预设权重分发流量（如自建节点 80%，备份节点 20%）。
- **高可用性 (High Availability)**:
    - **Automatic Failover (自动故障转移)**: 当主节点返回 5xx 错误或超时，Router 自动无缝重试下一个可用节点，用户对此毫无感知 (Seamless)。
    - **Circuit Breaker (熔断机制)**: 当某节点连续报错超过阈值，自动将其“熔断” (暂时踢出路由表) 60秒，避免拖累整体响应速度。
- **协议适配 (Protocol Adaptor - On Demand)**:
    - 仅在必要时（如 Client 是 OpenAI SDK，但目标是 Claude 原生 API）才启动极简的转换层。默认情况下，推荐使用各家原生 SDK 直连，Router 仅做鉴权与计费网关。
