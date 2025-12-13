仪表盘：全局概览。显示核心指标（请求数、成功率、总消费）、系统健康状态。
模型网络：核心路由配置。添加上游渠道（OpenAI, Gemini 等），设置 API Key、代理地址、模型映射、优先级、权重。
BurnGrid：账号购买渠道。
访问凭证：生成访问密钥。
演练场：测试中心。
风控雷达：监控日志、错误率、异常 IP。
日志审查：查看每条日志的消费记录，以及错误原因。
用户管理：管理下游用户（User）。创建账号、封禁用户、充值额度、查看用户组。
财务中心：查看充值记录、个人余额（如果是以用户视角登录）或系统营收（如果是管理员视角）。


所有页面统一存放在/crates/client/crates下面。仪表盘：client-dashboard。模型网络：client-models。BurnGrid：client-burngrid。访问凭证：client-access。演练场：client-playground。风控雷达：client-monitor。日志审查：client-log。用户管理：client-users。财务中心：client-finance。同时修改相应的访问路由


方案三：泛型透传与协议降级 (Generic Passthrough & Protocol Degradation)

  这个方案承认一个事实：网关永远追不上上游的变化。所以，网关不应该试图去“理解”每一个参数。

  1. 核心理念：不做全量解析 (Don't Parse Everything)

  目前的 BurnCloud 代码（以及 new-api）之所以脆弱，是因为它试图将 JSON 强转为 Rust Struct。
  OpenAI Request JSON -> Rust Struct -> Gemini Request JSON

  一旦上游多了一个字段，Rust Struct 没定义，这就丢了。

  改进方案：
  网关只解析路由所需的关键字段（如 model, stream, messages），其他所有字段全部作为 Extra 透传。

    1 // 伪代码：泛型请求结构
    2 struct GenericRequest {
    3     // 1. 核心字段：为了计费、流式控制、路由
    4     pub model: String,
    5     pub stream: bool,
    6     pub messages: Vec<Message>,
    7
    8     // 2. 透传字段：不管上游加了 topK, thinking, 还是 whatever，全都塞这里
    9     #[serde(flatten)]
   10     pub extra_params: HashMap<String, serde_json::Value>,
   11 }

  效果：
   * 如果 Google 加了 thinking_config，客户端发过来，网关不认识，但会把它存在 extra_params 里。
   * 在转发时，网关把 extra_params 原封不动地塞给上游。
   * 结果：无需改一行代码，新参数天然支持。

  2. 动态 Lua/WASM 插件 (Scriptable Plugins)

  如果仅仅是透传还不够（比如参数名变了，max_tokens 变成了 max_output_tokens），我们不想改 Rust 代码并重启服务。

  可以在 Rust 网关中嵌入一个轻量级脚本引擎（Lua 或 WASM）。

   * Nginx / APISIX / Kong 的做法：使用 Lua 脚本修改请求体。
   * Envoy 的做法：使用 WASM 过滤器。

  BurnCloud 实施方案：
  在 router 中集成 mlua (Lua) 或 rhai (Rust 脚本)。

  配置示例 (数据库中)：

    1 # channel_config 表中的 script 字段
    2 [script]
    3 type = "rhai"
    4 code = """
    5 // 这是一个热更新脚本，不需要重启 BurnCloud
    6 // 处理参数改名
    7 if "max_tokens" in req {
    8     req.generationConfig.maxOutputTokens = req.max_tokens;
    9     req.max_tokens = (); // 删除旧字段
   10 }
   11
   12 // 处理特殊逻辑
   13 if req.model == "gemini-1.5-pro" {
   14     req.safetySettings = [{"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_NONE"}];
   15 }
   16 """

  优势：
   * 确定性：脚本是逻辑确定的，不会像 AI 那样产生幻觉。
   * 热更新：发现 Google 变了，运维人员在后台贴一段脚本，立马生效。
   * 性能：Lua/Rhai/WASM 的性能损耗极低（微秒级），远好于调用 LLM。

  ---

  方案四：Schema 驱动的配置化 (Schema-Driven Configuration)

  这是一种声明式的方法。我们不写代码，也不写脚本，而是维护一份 Schema 映射表。

  很多现代 ETL 工具（如 Airbyte, Meltano）都用这种方式来处理成百上千种 API。

  核心逻辑：
  定义一个 DSL (Domain Specific Language) 来描述映射关系。

  Mapping Config (YAML):

    1 # 映射规则配置
    2 transforms:
    3   - move: ["max_tokens", "generationConfig.maxOutputTokens"]
    4   - move: ["temperature", "generationConfig.temperature"]
    5   - map_value:
    6       field: "messages[].role"
    7       map:
    8         "assistant": "model"
    9         "user": "user"
   10   - keep: ["*"] # 透传所有其他字段

  Rust 实现：
  写一个通用的解释器引擎，读取这个 YAML，对 JSON 进行变换。

  优势：
   * 安全：不像脚本那样能执行任意逻辑，只允许移动/改名/映射值。
   * 可视化：前端可以做一个“拖拽连线”的界面，让非技术运营也能配置参数映射。


在 BurnCloud 的界面设计（Jobs Design Philosophy）中，我们不能用冰冷的技术术语（如 Key Exchange, Handshake）。我们需要隐喻 (Metaphor)。

  我为你设计了 BurnGrid "Handshake" (握手) 交互方案，旨在让“达成共识”变得像交换名片一样自然。

  ---

  1. 核心隐喻：星际通行证 (Star Pass) & 握手 (Handshake)

  我们不再叫它“配置节点”，我们叫它 “建立连接 (Making a Connection)”。

  A. 你的身份：BurnID 卡片
   * 在 BurnGrid 界面，用户首先看到的是一张类似 Apple Wallet 登机牌 或 黑卡 的数字卡片。
   * 卡片内容:
       * BurnID: 你的唯一节点标识 (Hash)。
       * Availability: 闲置算力状态 (e.g., "Idle - 2x H100 Available").
       * Reputation: 你的信誉分 (基于历史履约率)。
   * 动作: 只有“复制链接”或“展示二维码”。

  B. 达成共识的过程：The Handshake UI

  想象两个企业主（或两个朋友）想要互联：

   1. 发起方 (Inviter):
       * 点击 "Invite Partner" (邀请伙伴)。
       * 系统生成一个 "Invitation Link" (邀请函)。这个链接包含了一次性的握手密钥和自己的公钥。
       * 界面隐喻: 这就像发送一个 AirDrop 请求。

   2. 接收方 (Invitee):
       * 点击链接或扫描二维码。
       * Dashboard 弹出一个精美的模态框：
          > "Elon's Node" 想要连接到您的 BurnGrid。
          > *   对方提供: GPT-4 (Via Azure), Llama-3 (Local)
          > *   对方请求: 您的闲置 GPU 算力
          >    信用额度*: 建议给予 1M Token 初始额度
       * 动作: 用户拖动一个滑块 "Trust & Connect" (信任并连接)。

   3. 共识达成 (Consensus Established):
       * 瞬间，界面上两点之间连成一条发光的线。
       * 视觉反馈: 屏幕上出现 "Handshake Complete" 字样，伴随轻微的触觉反馈（如果是在移动端）或悦耳的音效。
       * 这意味着：密钥交换已在后台自动完成，互信额度已建立。

  ---

  2. 界面呈现：信任圈可视化 (Trust Circle View)

  在 Model Network -> BurnGrid 页面：

   * 中心: 你自己的节点。
   * 内圈 (Trusted): 那些你手动“握手”过的节点（Guild 成员）。
       * 显示为绿色的实线连接。
       * 无需实时扣费，走“月结/信用”模式。
   * 外圈 (Public): 你从未见过的节点（通过 Marketplace 发现）。
       * 显示为灰色的虚线连接。
       * 必须“预付费”或“实时结算”。

  共识的调节器 (The Consensus Slider):
   * 点击任意一个连接的朋友，会弹出一个简单的控制面板：
       * 信任等级 (Trust Level): 滑块从 "Zero Trust" (实时结算) 到 "Full Trust" (无限制)。
       * 资源共享 (Resource Sharing): 开关 "Allow them to use my GPU" (允许他们用我的卡)。

  ---

  3. 文案与话术 (Copywriting)

  不要说：“配置 P2P 互信证书。”
  要说：“邀请朋友加入您的算力圈。” (Invite a friend to your Compute Circle.)

  不要说：“设置最大透支额度。”
  要说：“设置信任额度。” (Set Trust Limit.)

  不要说：“分布式账本同步成功。”
  要说：“你们已达成共识。” (You are now in sync.)

  ---

  4. 总结

  BurnGrid 在界面上达成共识的方式，就是将“社交网络的关注逻辑”引入到“服务器网络”中。

   * 建立连接 = 关注/互粉 (Follow/Connect)
   * 信任额度 = 亲密程度 (Close Friends)
   * 共识 = 握手成功 (Handshake)

  通过这种拟人化的设计，复杂的技术共识（密钥交换、白名单、路由表同步）被封装在了一个充满人情味的“握手”动作背后。这正是 Constitution 1.5
  要求的“优雅交互”。


# 导出下线分成数据报表
# 导出大客户的消费报表
# 增加ai分析功能，让ai去管理平台和渠道。用户做最终决策。
# 上游数据只提取部分，应该做到泛型透传与协议降级 (Generic Passthrough & Protocol Degradation)
# 目标是减少管理员的工作，管理员不应该每天去看日志错误报告，应该由系统来分析当前的渠道质量和错误情况。
# 账号的原始性，不允许管理员填写上游渠道
# 上游倍率问题，由平台全自动提供
# 平台与平台之间对接，错误过多，使用客户应该直接访问到平台
# 

# 路径请求为 /openai/deployments/{deployment-id}/chat/completions?api-version={version} 判定为azure
# 路径请求为 /v1beta/models/gemini-pro:generateContent 判定为google aistudio
# 路径请求为 /v1/projects/{PROJECT_ID}/locations/{REGION}/publishers/google/models/{MODEL_ID}:{ACTION} 判定为vertex

# [complete] service-models 删除模型
- 删除模型需要清理对应的文件，包含清理.1,.2,.3的后缀
- 例Qwen2.5-7B-Instruct-GGUF.1，Qwen2.5-7B-Instruct-GGUF.2，Qwen2.5-7B-Instruct-GGUF.3。
- 直接先去掉文件Qwen2.5-7B-Instruct-GGUF.guff的后缀，再删除Qwen2.5-7B-Instruct-GGUF*文件
# [complete] service-models 下载功能
# [complete] service-models 列出所有GUFF文件
# [complete] service-models 递归获取所有文件
# [complete] service-models 设置url host
# [complete] client-models 弹出功能修改
# [complete] client-models 添加模型页面
# [complete] service-models 增加 https://huggingface.co/api/models 对接
# [complete] service-ip cache 读取
# [complete] service-setting 初始化
# [complete] database-setting 
# [complete] service-ip 判断用户当前处于哪个网络环境
# [complete] client-models页面

# [complete] service-inference 基础架构
- 创建 crates/service/crates/service-inference
- 定义 InferenceConfig 和 InferenceService
- 实现 start_instance / stop_instance / get_status 逻辑
- 集成 llama-server 启动命令

# [complete] UI Design Refactor (Jobs Philosophy)
- Redesign Layout to be frameless with custom TitleBar (macOS/Windows adaptive).
- Redesign Sidebar with "Expert" and "BurnGrid" metaphors, using SVG icons.
- Redesign Dashboard to be minimalist ("Zen Mode"), hiding technical parameters.
- Add "Jobs Design Philosophy" to Constitution.

# [pending] Client - Dashboard Dynamic Data
- Connect the "Coding Expert" card to real running model data.
- Display "No Active Expert" state when no model is loaded.
- Connect "Neural Load" and "Context Memory" to real system metrics (partially done).

# [pending] Client - BurnGrid Integration
- Implement "BurnGrid" page to replace "Channels".
- Show network sharing status toggle (AirPlay metaphor).
- Implement "Universal Memory" placeholder page.
