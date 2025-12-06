# New API 功能深度分析报告 (Functional Analysis Report)

**日期**: 2025-12-07
**版本**: 基于 `new-api` 源码分析
**分析目标**: 深入理解 `new-api` 的架构设计、业务逻辑与核心功能，为 `BurnCloud` 开发提供参考。

---

## 1. 核心架构概述 (Architecture Overview)

`new-api` 是一个高性能、支持多渠道分发、商业化运营的 LLM 网关系统。其核心架构采用了经典的 **Router -> Controller -> Relay -> Adaptor** 模式，基于 Gin 框架构建。

*   **接入层 (Entry)**: 提供兼容 OpenAI (v1) 的标准接口，同时支持 Claude, Gemini 等原生协议转换。
*   **路由层 (Router)**: 基于用户组 (Group) 和模型 (Model) 的动态路由策略 (`Ability` 表)。
*   **分发层 (Relay)**: 处理请求体解析、鉴权、计费预扣，并分发给具体的渠道适配器。
*   **适配层 (Adaptor)**: 针对不同厂商 (OpenAI, Azure, Baidu, Ali, etc.) 实现协议规范化，屏蔽上游差异。

---

## 2. 核心功能模块 (Core Features)

### 2.1 极致的渠道兼容性 (Multi-Provider Support)
系统内置了对全球主流及特定区域模型厂商的深度适配。

*   **国际厂商**: OpenAI, Azure, Anthropic (Claude), Google (Gemini/PaLM), Midjourney, AWS Bedrock, Cohere, Mistral, DeepSeek.
*   **国内厂商**: 百度文心 (Baidu/BaiduV2), 阿里通义 (Ali), 智谱 (ChatGLM), 讯飞星火 (Xunfei), 腾讯混元 (Tencent), 360, 零一万物, 百川 (MokaAI), 字节豆包 (VolcEngine).
*   **专用/垂直模型**: Suno (音乐), Sora/Kling/Vidu (视频), Xinference, Ollama (本地模型).
*   **通用代理**: AIProxy, OpenRouter, API2GPT 等。

**代码证据**: `constant/channel.go` 定义了 56+ 种 `ChannelType`，且在 `relay/relay_adaptor.go` 中通过工厂模式 (`GetAdaptor`) 动态实例化。

### 2.2 高级流量路由系统 (Traffic Routing)
路由系统是其高性能的核心，不再是简单的轮询，而是基于 **Ability (能力)** 模型。

*   **Ability 模型**: 数据库表 `abilities` 记录了 `Group + Model + ChannelId` 的映射关系。
*   **分组路由 (Group Routing)**: 用户属于特定组 (如 `default`, `vip`)，渠道也绑定特定组。用户只能访问其组内有权限的渠道。
*   **负载均衡与故障转移**:
    *   **Priority (优先级)**: 优先使用高优先级渠道。
    *   **Weight (权重)**: 同优先级下，根据权重进行加权随机分发 (`model/ability.go:GetChannel`).
    *   **Auto Ban**: 自动禁用响应慢或报错的渠道。

### 2.3 商业化账户体系 (Commercial Account System)
专为运营设计的账户模型 (`model/user.go`)。

*   **身份认证**:
    *   支持账号密码、邮箱验证。
    *   **OAuth 集成**: GitHub, Discord, WeChat, Telegram, Linux.do, OIDC。
    *   **安全登录**: 支持 2FA (TOTP) 和 Passkey (WebAuthn/FIDO2)。
*   **额度管理 (Quota)**:
    *   基于 Token 的精准计费，支持 "预扣 -> 实际计算 -> 补偿" 机制。
    *   额度单位：500000 Quota = $1 (系统内部汇率)。
*   **充值与支付**:
    *   内置 **Stripe**, **Epay** (易支付), **Creem** 支付接口。
    *   支持 **Redemption Code** (兑换码) 生成与核销。
*   **邀请返利 (Affiliate)**:
    *   `AffCode`: 每个用户有独立邀请码。
    *   `AffQuota`: 邀请收益单独存储，支持划转到主余额。
    *   邀请关系记录在 `inviter_id`。

### 2.4 渠道管理增强 (Channel Management)
针对 API Key 管理的痛点进行了深度优化。

*   **多 Key 轮询 (Multi-Key)**: 单个渠道记录可以包含多个 API Key (换行分隔)，支持轮询 (Polling) 或随机 (Random) 策略 (`model/channel.go:GetNextEnabledKey`)。
*   **自动禁用**: 遇到 401/403 错误自动禁用 Key 或 Channel。
*   **标签系统 (Tags)**: 支持对渠道打 Tag，便于批量管理和筛选。
*   **参数覆写**: 支持在渠道层级覆写 HTTP Header 或 Body 参数 (`ParamOverride`, `HeaderOverride`)，解决某些特殊代理的鉴权需求。

### 2.5 异步任务系统 (Async Task System)
为了支持 Midjourney、Suno、Sora 等耗时较长的生成任务，引入了独立的任务队列模型 (`model/task.go`).

*   **流程**: 提交任务 -> 返回 TaskID -> 轮询/回调更新状态。
*   **状态机**: `QUEUED` -> `IN_PROGRESS` -> `SUCCESS` / `FAILURE`。
*   **统一接口**: 将不同平台的异步接口统一封装为类 OpenAI 或类 MJ 格式。

---

## 3. 数据模型深度解析 (Database Schema)

| 表名 | 核心字段 | 作用 |
| :--- | :--- | :--- |
| **users** | `quota`, `group`, `access_token`, `aff_quota`, `role` | 用户核心表，存储余额、权限和邀请信息。 |
| **channels** | `type`, `key` (多行), `base_url`, `models`, `group`, `priority`, `weight` | 上游配置表，定义如何连接外部服务。 |
| **abilities** | `group`, `model`, `channel_id`, `priority`, `weight` | **路由核心表**。系统启动时根据 Channel 自动生成，通过索引实现微秒级路由查询。 |
| **tokens** | `key`, `subnet`, `remain_quota` | 用户创建的应用级 API Key，支持子网段限制。 |
| **logs** | `user_id`, `type`, `content`, `quota`, `model_name` | 流水日志，用于审计和对账。 |
| **tasks** | `platform`, `action`, `status`, `progress`, `data` (JSON) | 异步任务状态存储。 |
| **redemptions** | `code`, `quota`, `status` | 充值兑换码。 |

---

## 4. 关键逻辑流程 (Key Logic Flows)

### 4.1 请求转发流程 (Request Relay)
1.  **Middleware**: 用户鉴权 (`Bearer Token`) -> 频率限制 -> 余额检查。
2.  **Controller**: `Relay` 函数接收请求，解析 Model 参数。
3.  **Router (Ability)**: 根据 `User.Group` + `Request.Model` 查询 `abilities` 表，按优先级和权重选出一个 `Channel`。
4.  **Adaptor**:
    *   `GetAdaptor(channel.Type)` 获取对应的适配器。
    *   **Request Conversion**: 将 OpenAI 格式请求转换为上游格式 (如转换成 Baidu SDK 格式)。
    *   **DoRequest**: 发送 HTTP 请求到 `Channel.BaseURL`。
    *   **Response Conversion**: 将上游响应流式转换为 OpenAI SSE 格式返回给客户端。
5.  **Billing**:
    *   流式响应结束后，统计 Token 数量。
    *   计算费用：`(Prompt + Completion) * ModelRatio * GroupRatio`。
    *   扣除用户余额，记录 `logs`。

---

## 5. 对 BurnCloud 的启示 (Insights for BurnCloud)

1.  **Ability 表的设计**: 极其精妙。将复杂的 JSON 配置打平成数据库关系表，使得路由查询变成了高效的 SQL 查询，且天然支持热重载（只需重新生成 Ability 表）。BurnCloud 的 Router 应该完全复刻这一设计。
2.  **Redb/Redis 双模**: New API 强依赖 Redis 进行缓存（User Quota, Token），但在没有 Redis 时也能降级查 DB。BurnCloud 计划引入的 `redb` 可以作为单机版的高性能替代。
3.  **Adaptor 接口化**: `relay/channel/` 下的目录结构非常清晰，每个厂商一个包，实现统一接口。这是 Rust `trait` 的完美应用场景。
4.  **异步任务**: 对于未来的视频/生图功能，必须尽早设计 `Task` 表，不能试图用同步 HTTP 连接去 hold 住几分钟的生成过程。

---

*本报告基于 `data/new-api-main` 源码生成。*