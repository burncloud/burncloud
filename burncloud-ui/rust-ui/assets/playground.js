(() => {
  const root = document.querySelector("[data-playground]");
  if (!root) return;

  const elements = {
    model: root.querySelector("#model-select"),
    tierButtons: [...root.querySelectorAll("[data-tier]")],
    tierDescription: root.querySelector("#tier-description"),
    summary: root.querySelector("#playground-summary"),
    temperature: root.querySelector("#temperature"),
    temperatureValue: root.querySelector("#temperature-value"),
    maxTokens: root.querySelector("#max-tokens"),
    maxTokensValue: root.querySelector("#max-tokens-value"),
    systemPrompt: root.querySelector("#system-prompt"),
    userPrompt: root.querySelector("#user-prompt"),
    codeTabs: [...root.querySelectorAll("[data-code-tab]")],
    codeOutput: root.querySelector("#code-output"),
    copyButton: root.querySelector("#copy-code"),
    copyStatus: root.querySelector("#copy-status"),
    runButton: root.querySelector("#run-inference"),
    clearButton: root.querySelector("#clear-output"),
    response: root.querySelector("#response-output"),
    streamStatus: root.querySelector("#stream-status"),
    stats: root.querySelector("#execution-stats"),
    receipt: root.querySelector("#execution-receipt"),
    statTtft: root.querySelector("#stat-ttft"),
    statTotal: root.querySelector("#stat-total"),
    statTokens: root.querySelector("#stat-tokens"),
    statCost: root.querySelector("#stat-cost"),
  };

  let activeTier = "standard";
  let activeCodeTab = "curl";
  let running = false;
  const tierLabels = { economy: "经济级", standard: "标准级", performance: "性能级" };

  const escapeJson = (value) => JSON.stringify(value).slice(1, -1);
  const selectedModel = () => elements.model?.value || "";
  const snippets = () => {
    const model = selectedModel();
    const system = escapeJson(elements.systemPrompt.value);
    const user = escapeJson(elements.userPrompt.value);
    const temperature = Number(elements.temperature.value);
    const maxTokens = Number(elements.maxTokens.value);
    return {
      curl: `curl https://api.burncloud.io/v1/chat/completions \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer $BURNCLOUD_API_KEY" \\
  -H "X-BurnCloud-Tier: ${activeTier}" \\
  -d '{
    "model": "${escapeJson(model)}",
    "messages": [
      {"role": "system", "content": "${system}"},
      {"role": "user", "content": "${user}"}
    ],
    "temperature": ${temperature},
    "max_tokens": ${maxTokens}
  }'`,
      python: `import os
from openai import OpenAI

client = OpenAI(
    api_key=os.environ["BURNCLOUD_API_KEY"],
    base_url="https://api.burncloud.io/v1",
    default_headers={"X-BurnCloud-Tier": "${activeTier}"}
)

response = client.chat.completions.create(
    model="${escapeJson(model)}",
    messages=[
        {"role": "system", "content": "${system}"},
        {"role": "user", "content": "${user}"}
    ],
    temperature=${temperature},
    max_tokens=${maxTokens}
)
print(response.choices[0].message.content)`,
      node: `import OpenAI from "openai";

const openai = new OpenAI({
  apiKey: process.env.BURNCLOUD_API_KEY,
  baseURL: "https://api.burncloud.io/v1",
  defaultHeaders: { "X-BurnCloud-Tier": "${activeTier}" }
});

const response = await openai.chat.completions.create({
  model: "${escapeJson(model)}",
  messages: [
    { role: "system", content: "${system}" },
    { role: "user", content: "${user}" }
  ],
  temperature: ${temperature},
  max_tokens: ${maxTokens}
});
console.log(response.choices[0].message.content);`,
    };
  };

  const updateCode = () => {
    elements.codeOutput.textContent = snippets()[activeCodeTab];
  };
  const updateSummary = () => {
    const model = selectedModel() || "model-id";
    elements.summary.textContent = `${model} · ${tierLabels[activeTier]} · 真实数据库路由`;
    elements.temperatureValue.textContent = Number(elements.temperature.value).toFixed(1);
    elements.maxTokensValue.textContent = elements.maxTokens.value;
    updateCode();
  };
  const clearOutput = () => {
    elements.response.replaceChildren();
    const placeholder = document.createElement("span");
    placeholder.className = "output-placeholder";
    placeholder.textContent = "点击“运行真实推理”后执行数据库中配置的模型路由。";
    elements.response.append(placeholder);
    elements.stats.hidden = true;
    elements.receipt.hidden = true;
    elements.streamStatus.hidden = true;
  };
  const errorMessage = (value, status) => {
    if (value?.error?.message) return value.error.message;
    if (typeof value?.message === "string") return value.message;
    return `请求失败 (${status})`;
  };

  const runInference = async () => {
    if (running) return;
    const model = selectedModel();
    const tokenRef = root.dataset.tokenRef;
    const userPrompt = elements.userPrompt.value.trim();
    if (!model || !tokenRef || !userPrompt) {
      elements.response.textContent = "请选择模型、确认有效 API 密钥并输入提示词。";
      return;
    }

    const messages = [];
    const systemPrompt = elements.systemPrompt.value.trim();
    if (systemPrompt) messages.push({ role: "system", content: systemPrompt });
    messages.push({ role: "user", content: userPrompt });
    const payload = {
      token_ref: tokenRef,
      model,
      messages,
      temperature: Number(elements.temperature.value),
      max_tokens: Number(elements.maxTokens.value),
    };

    running = true;
    elements.runButton.disabled = true;
    elements.runButton.querySelector("span").textContent = "真实路由执行中…";
    elements.streamStatus.hidden = false;
    elements.stats.hidden = true;
    elements.receipt.hidden = true;
    elements.response.textContent = "正在等待 BurnCloud 数据面返回上游响应…";
    const started = performance.now();
    try {
      const response = await fetch("/api/playground/chat", {
        method: "POST",
        credentials: "same-origin",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify(payload),
      });
      const raw = await response.text();
      let data;
      try {
        data = JSON.parse(raw);
      } catch {
        throw new Error(raw || `后端返回了无法解析的响应 (${response.status})`);
      }
      if (!response.ok) throw new Error(errorMessage(data, response.status));

      const content = data?.choices?.[0]?.message?.content;
      if (typeof content !== "string") throw new Error("响应中没有 assistant message content");
      const totalMs = Math.round(performance.now() - started);
      const usage = data.usage || {};
      const promptTokens = Number(usage.prompt_tokens || 0);
      const completionTokens = Number(usage.completion_tokens || 0);
      const option = elements.model.selectedOptions[0];
      const inputPrice = Number(option?.dataset.inputPrice || 0);
      const outputPrice = Number(option?.dataset.outputPrice || 0);
      const cost = (promptTokens * inputPrice + completionTokens * outputPrice) / 1_000_000;
      const channel = response.headers.get("x-channel-id") || "未返回";
      const routedModel = response.headers.get("x-model-id") || model;

      elements.response.textContent = content;
      elements.statTtft.textContent = "非流式";
      elements.statTotal.textContent = `${totalMs} ms`;
      elements.statTokens.textContent = `${promptTokens} / ${completionTokens}`;
      elements.statCost.textContent = `$${cost.toFixed(6)}`;
      elements.stats.hidden = false;
      elements.receipt.querySelector("code").textContent = `channel=${channel} · model=${routedModel}`;
      elements.receipt.hidden = false;
    } catch (error) {
      elements.response.textContent = `请求失败：${error instanceof Error ? error.message : String(error)}`;
    } finally {
      running = false;
      elements.streamStatus.hidden = true;
      elements.runButton.disabled = false;
      elements.runButton.querySelector("span").textContent = "运行真实推理";
    }
  };

  elements.model?.addEventListener("change", () => {
    const url = new URL(window.location.href);
    if (selectedModel()) url.searchParams.set("model", selectedModel());
    window.history.replaceState({}, "", url);
    updateSummary();
  });
  elements.temperature.addEventListener("input", updateSummary);
  elements.maxTokens.addEventListener("input", updateSummary);
  elements.systemPrompt.addEventListener("input", updateCode);
  elements.userPrompt.addEventListener("input", updateCode);
  elements.tierButtons.forEach((button) => button.addEventListener("click", () => {
    activeTier = button.dataset.tier;
    elements.tierButtons.forEach((item) => {
      const selected = item === button;
      item.classList.toggle("active", selected);
      item.setAttribute("aria-pressed", String(selected));
    });
    elements.tierDescription.textContent = `${tierLabels[activeTier]}将写入外部 API 示例；控制台测试由后端当前路由策略执行。`;
    updateSummary();
  }));
  elements.codeTabs.forEach((tab) => tab.addEventListener("click", () => {
    activeCodeTab = tab.dataset.codeTab;
    elements.codeTabs.forEach((item) => item.setAttribute("aria-selected", String(item === tab)));
    updateCode();
  }));
  elements.copyButton.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(elements.codeOutput.textContent);
      elements.copyButton.querySelector("span").textContent = "已复制";
      elements.copyStatus.textContent = "代码已复制到剪贴板";
    } catch {
      elements.copyStatus.textContent = "浏览器未允许访问剪贴板";
    }
    window.setTimeout(() => {
      elements.copyButton.querySelector("span").textContent = "复制代码";
      elements.copyStatus.textContent = "";
    }, 2000);
  });
  elements.clearButton.addEventListener("click", clearOutput);
  elements.runButton.addEventListener("click", runInference);
  updateSummary();
})();
