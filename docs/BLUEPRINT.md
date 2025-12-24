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
抛弃传统运维面板的拥挤感，采用 "Zen Mode"（禅模式）。只在需要时展示细节，平时提供呼吸感的系统健康状态概览。

### 深度规划
- **动态状态卡片 (Live Status Cards)**:
    - **Neural Load (神经负载)**: 实时 CPU/GPU 使用率，使用波形图展示。
    - **Synapses (活跃连接)**: 当前并发连接数 (QPS)，动态数字跳动。
    - **Memory (上下文记忆)**: 向量数据库或 KV 缓存的占用情况。
- **专家在线状态 (Active Expert)**:
    - 展示当前加载的主模型（如 "Llama-3-70B is Active"）。
    - 既然是“专家隐喻”，这里应该显示一个代表该模型的 Avatar 或图标。
- **快速入口 (Quick Actions)**:
    - "Summon Expert" (跳转模型加载)。
    - "Emergency Stop" (一键切断所有连接，Panic Button)。

---

## 2. 模型网络 (Model Network)
**Codename:** *The Armory*
**Module:** `client-models` & `service-models`

### 核心理念
本地主权 (Local Sovereignty)。管理本地的大脑（LLMs）和连接云端的大脑。

### 深度规划
- **本地仓库 (Local Repository)**:
    - **GGUF Manager**: 自动扫描 `models/` 目录，解析 GGUF 元数据（量化等级、参数量）。
    - **Version Control**: 智能处理同名模型的不同版本（如 `.1`, `.2` 后缀清理）。
    - **One-Click Serve**: 针对任意 GGUF 文件，一键启动 `llama-server` 实例。
- **云端市场 (Cloud Marketplace)**:
    - **Mirror Integration**: 自动检测网络环境，中国用户自动走 `hf-mirror.com`。
    - **Model Cards**: 渲染 HuggingFace Readme，提供富文本介绍。
    - **断点续传**: 集成 Aria2 或类似下载器，提供可视化的下载进度和暂停/恢复功能。
- **推理配置 (Inference Tuning)**:
    - 图形化配置 Context Window (n_ctx), GPU Layers (n_gpu_layers), Threads。

---

## 3. BurnGrid (分布式算力网)
**Codename:** *The Constellation*
**Module:** `client-burngrid`

### 核心理念
基于“握手”与“信任”的社交化算力共享。拒绝冷冰冰的 IP 配置。

### 深度规划
- **星际通行证 (Star Pass)**:
    - 用户的数字身份卡片，包含 BurnID (Node Hash)、信誉分、闲置算力状态。
    - 支持生成“邀请函” (Invite Link/QR Code)。
- **信任圈拓扑 (Trust Circle Visualization)**:
    - **内圈 (Inner Circle)**: 强信任节点（朋友/公司内部），绿色实线连接，信用记账。
    - **外圈 (Outer Circle)**: 弱信任节点（公网/市场），灰色虚线连接，实时结算。
    - **动画交互**: 节点之间的连线会有粒子流动，代表数据传输。
- **握手协议 UI (Handshake UI)**:
    - 收到连接请求时，弹出精美模态框，显示对方身份和算力报价。
    - **信任滑块**: 用户拖动滑块决定信任等级 (从 "Zero Trust" 到 "Full Access")。

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
