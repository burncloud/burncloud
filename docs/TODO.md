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


# 导出下线分成数据报表
# 导出大客户的消费报表
# 增加ai分析功能，让ai去管理平台和渠道。用户做最终决策。
# 上游数据只提取部分，应该做到泛型透传与协议降级 (Generic Passthrough & Protocol Degradation)
# 目标是减少管理员的工作，管理员不应该每天去看日志错误报告，应该由系统来分析当前的渠道质量和错误情况。
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

