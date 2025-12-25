# BurnCloud Product Roadmap & Feature Specification

> **Last Updated:** 2025-12-24
> **Status:** Active Planning
> **Philosophy:** Jobs Design (Metaphor-driven), Local First, Performance Oriented.

This document outlines the detailed functional planning for the 10 core modules of the BurnCloud Client.

---

## 1. 仪表盘 (Dashboard)
**Codename:** *Zen Command*
**Module:** `client-dashboard`

### 核心理念
抛弃传统运维面板的拥挤感，采用 "Zen Mode"（禅模式）。只在需要时展示细节，平时提供呼吸感的系统健康状态概览。目前设计融合了企业级控制台的严谨与 Consumer App 的流畅。

### 深度规划
- **成本与吞吐量监控 (Cost & Throughput Monitor)**:
    - **今日流水**: 实时展示今日 API 调用的估算成本 (USD) 和 Token 消耗量。
    - **实时交易流**: 顶部的呼吸灯指示器，直观展示当前的并发请求状态 (QPS)。
- **上游健康矩阵 (Upstream Health Matrix)**:
    - **供应商卡片**: 将 AWS, Google, Azure 等上游渠道可视化为独立的健康卡片。
    - **状态指示**: 使用颜色 (Green/Yellow/Red) 和脉冲动画展示各渠道的连通性和错误率。
    - **账户池监控**: 显示每个供应商下活跃的 API Key/Account 数量 (e.g., "AWS: 1,204 Active Accounts")。
- **动态数据集成 (Real-time Integration)**:
    - **[Next Step]**: 替换当前的硬编码 Mock 数据 (如 "$128,432.00")。
    - **数据源**: 从 `Router` 获取实时的 Prometheus/Metrics 数据流。
- **风控雷达缩略图 (Risk Radar Mini)**:
    - 侧边栏实时滚动展示被拦截的异常请求或高风险行为，点击可跳转至完整风控页面。

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
- **本地仓库 (Local Repository)**:
    - **GGUF Manager**: 扫描并管理本地 `models/` 目录下的量化模型。
    - **One-Click Serve**: 针对 GGUF 文件一键启动本地推理服务 (llama-server)，并自动注册到路由表中。
- **统一接口 (Unified Interface)**:
    - **Model Standardization**: 无论上游是 Azure 还是本地，在列表中统一展示为标准的模型卡片 (Context Window, Max Tokens)。
    - **Latency Test**: 一键测试所有已配置模型的连通性和延迟 (Ping)。

---

## 3. BurnGrid (分布式算力网)
**Codename:** *The Marketplace*
**Module:** `client-burngrid`

### 核心理念
全球算力采购与聚合中心。不再需要分别去 AWS、Azure 注册账号，BurnGrid 提供一站式的全球账号采购与接入服务，构建您的全球算力组合。

### 深度规划
- **全球资源采购 (Global Procurement)**:
    - **Account Brokerage**: 直接在界面上浏览并采购带有高额 Quota 的 AWS Bedrock, Azure OpenAI, Google Vertex 企业级账号。
    - **Instant Access**: 购买即用，自动将购买的凭证 (Credentials) 注入到系统的 Vault 中。
- **资源池管理 (Resource Pool)**:
    - **Active Nodes**: 可视化展示当前拥有的所有云端节点状态 (e.g., "AWS us-east-1: Active", "Azure Japan: Active")。
    - **Quota Monitor**: 实时监控各账号的 TPM (Tokens Per Minute) 和 RPM (Requests Per Minute) 限制，自动预警。
- **统一计费 (Unified Billing)**:
    - **Single Wallet**: 使用单一的 BurnCloud 钱包支付所有上游供应商的费用。
    - **Cost Optimization**: 智能建议购买哪种账号组合最划算。

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
**Module:** `client-radar`

### 核心理念
看不见的防线。主动防御与合规。

### 深度规划
- **实时拦截地图 (Live Threat Map)**:
    - 使用世界地图可视化展示被拦截的请求来源 (GeoIP)。
    - 闪烁红点表示潜在的攻击或违规访问。
- **内容安全策略 (Content Safety Policy)**:
    - **敏感词库管理**: 本地维护敏感词列表，支持正则。
    - **模型侧过滤**: 配置 LLM 的安全等级 (Safety Settings, e.g., Gemini HARM_CATEGORY)。
- **审计追踪**:
    - 列出最近触发风控规则的请求详情 (User, IP, Triggered Rule)。

---

## 7. 日志审查 (Log Review)
**Codename:** *Blackbox*
**Module:** `client-log`

### 核心理念
AI 驱动的洞察，而非枯燥的文本堆砌。

### 深度规划
- **AI 每日简报 (AI Daily Brief)**:
    - 每天自动分析日志，生成自然语言摘要：“昨日系统运行平稳，拦截了 5 次异常请求，GPT-4 渠道响应时间略有增加。”
- **故障回溯 (Traceback)**:
    - 当发生 500 错误时，提供完整的 Request ID 链路视图。
    - 关联展示当时的系统负载和网络状态。
- **智能筛选**:
    - "Show only Errors", "Show Latency > 2s", "Show Cost > $1"。

---

## 8. 用户管理 (User Management)
**Codename:** *Organization*
**Module:** `client-users` & `client-register`

### 核心理念
企业级 RBAC (基于角色的访问控制)。

### 深度规划
- **人员花名册 (Roster)**:
    - 列表展示系统内所有用户。
    - 状态指示: Online, Offline, Banned。
- **角色分配 (Role Assignment)**:
    - **Admin**: 全权访问。
    - **Developer**: 仅访问 Playground, API Keys, Logs。
    - **User**: 仅访问 Chat 接口。
- **邀请机制**:
    - 生成一次性注册链接，通过 BurnGrid 网络发送给新成员。

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
