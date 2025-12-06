# BurnCloud 用户指南 (User Guide)

欢迎使用 **BurnCloud (奔云)**！本文档将指导您如何配置和使用 BurnCloud 来管理您的 AI 模型接口。

---

## 1. 快速入门 (Quick Start)

BurnCloud 是一个“二合一”的工具：
1.  **网关 (Router)**: 运行在后台 (默认端口 3000)，负责处理所有的 API 请求。
2.  **客户端 (Client)**: 一个可视化的桌面程序，用于管理网关配置、查看日志和部署本地模型。

启动程序后，您可以通过标准 OpenAI SDK 访问网关：

```python
import openai

client = openai.OpenAI(
    base_url="http://127.0.0.1:3000/v1",
    api_key="sk-burncloud-demo" # 默认生成的测试 Token
)
```

---

## 2. 渠道管理 (Channel Management)

渠道 (Upstream) 是指您拥有的真实的 AI 服务商接口，例如 OpenAI 账号、Azure 部署或本地运行的模型。

### 添加渠道
1.  进入 **"渠道管理"** 页面。
2.  点击右上角 **"➕ 添加渠道"**。
3.  填写表单：
    *   **名称**: 给渠道起个名字 (如 "My OpenAI Pro")。
    *   **类型**: 选择服务商 (OpenAI, Claude, Gemini, etc.)。
    *   **地址**: API 的基地址 (如 `https://api.openai.com`)。
    *   **密钥**: 您的真实 API Key。
    *   **模型重定向**: (可选) 将请求的模型名映射为另一个名字。

### 负载均衡
您可以添加多个相同类型的渠道。BurnCloud 会自动在它们之间进行**轮询 (Round-Robin)** 负载均衡。如果某个渠道报错 (5xx 或 网络错误)，系统会自动尝试下一个可用的渠道 (**故障转移**)。

---

## 3. 令牌管理 (Token Management)

为了安全起见，您不应该直接在代码中使用真实的厂商 API Key。BurnCloud 允许您创建**虚拟令牌 (Virtual Token)**。

1.  进入 **"令牌管理"** 页面。
2.  点击 **"➕ 创建令牌"**。
3.  设置：
    *   **额度**: 限制该令牌能使用的最大金额或次数 (支持无限)。
    *   **分组**: (高级) 限制该令牌只能访问特定的渠道分组。
4.  将生成的 `sk-burncloud-xxx` 分发给您的用户或应用。

---

## 4. 本地模型部署 (Local Inference)

BurnCloud 内置了对 `GGUF` 格式大模型的支持，允许您在本地运行开源 LLM。

### 下载模型
1.  进入 **"模型管理"** 页面。
2.  点击 **"➕ 添加模型"**。
3.  在搜索框中输入模型名称 (如 `Qwen/Qwen2.5-7B-Instruct-GGUF`)。
4.  选择模型并点击 **"下载"**。
5.  在模型卡片上点击 **"📄 详情"**，选择具体的 `.gguf` 量化版本进行下载。

### 启动服务
1.  下载完成后，在模型卡片上点击 **"🚀 部署"**。
2.  配置参数：
    *   **端口**: 服务监听端口 (默认 8080)。
    *   **Context Size**: 上下文长度 (如 4096)。
    *   **GPU 层数**: 如果您有显卡，设置层数以加速推理 (设为 -1 使用全部显存)。
3.  点击 **"启动服务"**。
4.  成功后，模型状态会变为 **"🟢 运行中"**。

### 调用本地模型
本地模型启动后，会自动注册到网关。
您可以使用请求中的 `model` 参数来指定调用它。默认 ID 格式为 `local-{model_id}`。

```bash
curl http://127.0.0.1:3000/v1/chat/completions \
  -H "Authorization: Bearer sk-burncloud-demo" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "local-Qwen2.5-7B-Instruct",
    "messages": [{"role": "user", "content": "你好！"}]
  }'
```

---

## 5. 常见问题 (FAQ)

**Q: 网关端口被占用了怎么办？**
A: 您可以在启动时通过环境变量或配置文件修改端口。默认使用 3000。

**Q: 支持哪些本地模型？**
A: BurnCloud 底层使用 `llama.cpp` (llama-server)，因此支持所有兼容 GGUF 格式的模型 (Llama 3, Qwen 2, Mistral 等)。

**Q: 日志在哪里看？**
A: 客户端的 **"仪表盘"** 页面提供实时的调用日志流，包含耗时、Token 消耗和错误信息。
