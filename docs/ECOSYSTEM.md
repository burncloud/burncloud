# BurnCloud 生态集成指南 (Ecosystem Integration Guide)

BurnCloud 的网关 (Router) 完全兼容 OpenAI API 标准，因此可以无缝集成到任何支持 OpenAI 的第三方应用中。

本文档将指导您如何将 BurnCloud 连接到流行的开源 Chat UI。

---

## 1. 准备工作

在开始之前，请确保：
1.  BurnCloud 已启动 (默认地址 `http://127.0.0.1:3000`)。
2.  您已创建一个令牌 (默认测试令牌: `sk-burncloud-demo`)。
3.  您已在 BurnCloud 中配置了至少一个可用的渠道（云端或本地模型）。

---

## 2. 集成 NextChat (ChatGPT-Next-Web)

[NextChat](https://github.com/ChatGPTNextWeb/ChatGPT-Next-Web) 是目前最流行的轻量级 Chat UI 之一。

### 配置方式
在启动 NextChat 时（无论是 Docker 还是 Vercel 部署），设置以下环境变量：

*   **BASE_URL**: `http://127.0.0.1:3000` (注意：不要加 `/v1`)
*   **OPENAI_API_KEY**: `sk-burncloud-demo`
*   **CUSTOM_MODELS**: `+local-Qwen2.5-7B-Instruct` (可选，添加您的本地模型名称)

**Docker 示例**:
```bash
docker run -d -p 3001:3000 \
   -e OPENAI_API_KEY=sk-burncloud-demo \
   -e BASE_URL=http://host.docker.internal:3000 \
   yidadaa/chatgpt-next-web
```

---

## 3. 集成 LobeChat

[LobeChat](https://github.com/lobehub/lobe-chat) 是一个高性能的现代化 Chat UI，支持插件和语音。

### 配置方式
LobeChat 支持直接在设置界面配置自定义 OpenAI 代理。

1.  进入 **设置 (Settings)** -> **语言模型 (Language Model)**。
2.  找到 **OpenAI** 卡片。
3.  开启 **Proxy Url** 开关，并填入: `http://127.0.0.1:3000/v1` (注意：LobeChat 通常需要 `/v1` 后缀)。
4.  在 **API Key** 中填入: `sk-burncloud-demo`。
5.  点击 **检查 (Check)** 验证连接。

---

## 4. 集成 Open WebUI

[Open WebUI](https://github.com/open-webui/open-webui) (原 Ollama WebUI) 是功能最强大的本地 LLM 界面。

### 配置方式
Open WebUI 默认支持 OpenAI 兼容后端。

**Docker 示例**:
```bash
docker run -d -p 8080:8080 \
  -e OPENAI_API_BASE_URL=http://host.docker.internal:3000/v1 \
  -e OPENAI_API_KEY=sk-burncloud-demo \
  ghcr.io/open-webui/open-webui:main
```

---

## 5. 常见问题

**Q: 为什么 NextChat 报错 "Failed to fetch"?**
A: 如果您在 Docker 中运行 NextChat，而 BurnCloud 在宿主机运行，请将 `BASE_URL` 设置为 `http://host.docker.internal:3000` 而不是 `127.0.0.1`。

**Q: 模型列表没有显示我的本地模型？**
A: BurnCloud 的 `/v1/models` 接口目前返回静态列表（为了性能）。您可以在客户端手动输入模型名称 (如 `local-model-id`)，或者等待我们后续更新动态模型列表支持。
