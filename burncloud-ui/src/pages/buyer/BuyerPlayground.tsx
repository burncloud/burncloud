import React, { useState } from 'react';
import {
  Play,
  Copy,
  Check,
  RotateCcw,
  Code2,
  ShieldCheck,
  Cpu
} from 'lucide-react';
import {
  BCPageHeader,
  BCCard,
  BCButton
} from '@/components/ui';
import { WORKBENCH_MODELS, ModelItem } from '@/data/workbenchData';
import { useTranslation } from '@/i18n/I18nContext';

export function BuyerPlayground() {
  const { t } = useTranslation();
  const [selectedModel, setSelectedModel] = useState<ModelItem>(WORKBENCH_MODELS[0]);
  const [selectedTier, setSelectedTier] = useState<'Economy' | 'Standard' | 'Performance'>('Standard');
  const [systemPrompt, setSystemPrompt] = useState<string>('You are an expert full-stack engineer and distributed systems architect. Provide precise, actionable technical answers.');
  const [userPrompt, setUserPrompt] = useState<string>('Explain how hardware attestation guarantees secure multi-tenant LLM token generation.');
  const [temperature, setTemperature] = useState<number>(0.7);
  const [maxTokens, setMaxTokens] = useState<number>(2048);

  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [streamedResponse, setStreamedResponse] = useState<string>('');
  const [executionStats, setExecutionStats] = useState<{
    ttftMs: number;
    totalMs: number;
    promptTokens: number;
    completionTokens: number;
    costUsd: number;
    hardwareReceiptId: string;
  } | null>(null);

  const [activeCodeTab, setActiveCodeTab] = useState<'curl' | 'python' | 'node'>('curl');
  const [copiedCode, setCopiedCode] = useState(false);

  const handleRunInference = () => {
    setIsLoading(true);
    setStreamedResponse('');
    setExecutionStats(null);

    const fullText = `Hardware attestation provides cryptographic verification that inference workloads run strictly inside an isolated hardware enclave (e.g. NVIDIA H100 Confidential Computing or Nitro TPM enclaves) without memory inspection or host tampering.\n\nKey architectural pillars:\n1. Root of Trust: An encrypted attestation signature is generated at the silicon level prior to weight loading.\n2. Zero-Retention Memory: Activations and KV caches are wiped immediately upon stream termination.\n3. Cryptographic Token Receipts: Every completion chunk is signed with the enclave private key, proving zero prompt logging or intermediary proxy interception.`;

    let currentIndex = 0;
    const startTime = performance.now();

    const interval = setInterval(() => {
      currentIndex += 12;
      if (currentIndex >= fullText.length) {
        setStreamedResponse(fullText);
        clearInterval(interval);
        setIsLoading(false);
        const totalMs = Math.round(performance.now() - startTime + 280);
        setExecutionStats({
          ttftMs: selectedTier === 'Performance' ? 112 : selectedTier === 'Standard' ? 148 : 230,
          totalMs,
          promptTokens: 42,
          completionTokens: 184,
          costUsd: Number(((42 * selectedModel.inputPrice1M + 184 * selectedModel.outputPrice1M) / 1000000).toFixed(6)),
          hardwareReceiptId: 'rcpt_sec_994a_nitro'
        });
      } else {
        setStreamedResponse(fullText.slice(0, currentIndex));
      }
    }, 35);
  };

  const getCurlSnippet = () => `curl https://api.burncloud.io/v1/chat/completions \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer $BURNCLOUD_API_KEY" \\
  -H "X-BurnCloud-Tier: ${selectedTier.toLowerCase()}" \\
  -d '{
    "model": "${selectedModel.id}",
    "messages": [
      {"role": "system", "content": "${systemPrompt.replace(/"/g, '\\"')}"},
      {"role": "user", "content": "${userPrompt.replace(/"/g, '\\"')}"}
    ],
    "temperature": ${temperature},
    "max_tokens": ${maxTokens}
  }'`;

  const getPythonSnippet = () => `from openai import OpenAI

# BurnCloud endpoint is 100% drop-in compatible with standard OpenAI SDKs
client = OpenAI(
    api_key="demo-api-key",
    base_url="https://api.burncloud.io/v1",
    default_headers={"X-BurnCloud-Tier": "${selectedTier.toLowerCase()}"}
)

response = client.chat.completions.create(
    model="${selectedModel.id}",
    messages=[
        {"role": "system", "content": "${systemPrompt.replace(/"/g, '\\"')}"},
        {"role": "user", "content": "${userPrompt.replace(/"/g, '\\"')}"}
    ],
    temperature=${temperature},
    max_tokens=${maxTokens}
)

print(response.choices[0].message.content)`;

  const getNodeSnippet = () => `import OpenAI from "openai";

const openai = new OpenAI({
  apiKey: process.env.BURNCLOUD_API_KEY,
  baseURL: "https://api.burncloud.io/v1",
  defaultHeaders: {
    "X-BurnCloud-Tier": "${selectedTier.toLowerCase()}"
  }
});

async function main() {
  const completion = await openai.chat.completions.create({
    model: "${selectedModel.id}",
    messages: [
      { role: "system", content: "${systemPrompt.replace(/"/g, '\\"')}" },
      { role: "user", content: "${userPrompt.replace(/"/g, '\\"')}" }
    ],
    temperature: ${temperature},
    max_tokens: ${maxTokens}
  });

  console.log(completion.choices[0].message.content);
}

main();`;

  const handleCopyCode = () => {
    const text =
      activeCodeTab === 'curl'
        ? getCurlSnippet()
        : activeCodeTab === 'python'
        ? getPythonSnippet()
        : getNodeSnippet();
    navigator.clipboard.writeText(text);
    setCopiedCode(true);
    setTimeout(() => setCopiedCode(false), 2000);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.buyer.playground.title}
        subtitle={t.buyer.playground.subtitle}
        conclusion={{
          text: `${t.buyer.playground.conclusion} (${selectedModel.name} • ${selectedTier})`,
          type: 'healthy'
        }}
      />

      {/* Main Grid: Left Controls, Center Prompt & Response, Right Code Snippet */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left Col: Model & Parameter Settings */}
        <div className="lg:col-span-4 space-y-4">
          <BCCard className="p-5 space-y-4">
            <h3 className="text-xs font-bold text-gray-900 uppercase tracking-wider font-mono flex items-center gap-1.5">
              <Cpu className="w-4 h-4 text-gray-700" />
              <span>{t.buyer.playground.modelSelect} & {t.buyer.playground.routingTier}</span>
            </h3>

            {/* Model Select */}
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-gray-700">{t.buyer.playground.modelSelect}</label>
              <select
                value={selectedModel.id}
                onChange={(e) => {
                  const m = WORKBENCH_MODELS.find((x) => x.id === e.target.value);
                  if (m) setSelectedModel(m);
                }}
                className="w-full h-9 bg-gray-50 border border-gray-200 rounded-xl px-3 text-xs text-gray-900 font-medium focus:bg-white focus:outline-none focus:ring-2 focus:ring-gray-900/10 focus:border-gray-900 cursor-pointer"
              >
                {WORKBENCH_MODELS.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name} (${m.inputPrice1M} / ${m.outputPrice1M})
                  </option>
                ))}
              </select>
            </div>

            {/* Performance Tier Selector */}
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-gray-700">{t.buyer.playground.routingTier}</label>
              <div className="grid grid-cols-3 gap-1.5 bg-gray-100 p-1 rounded-xl">
                {(['Economy', 'Standard', 'Performance'] as const).map((tier) => (
                  <button
                    key={tier}
                    type="button"
                    onClick={() => setSelectedTier(tier)}
                    className={`py-1.5 text-xs font-medium rounded-lg transition-all cursor-pointer ${
                      selectedTier === tier
                        ? 'bg-white text-gray-950 font-bold shadow-xs'
                        : 'text-gray-600 hover:text-gray-900'
                    }`}
                  >
                    {tier === 'Economy' ? t.buyer.playground.tierEconomy.split(' ')[0] : tier === 'Standard' ? t.buyer.playground.tierStandard.split(' ')[0] : t.buyer.playground.tierPerformance.split(' ')[0]}
                  </button>
                ))}
              </div>
              <p className="text-[10px] text-gray-500 mt-1">
                {selectedTier === 'Economy' && t.buyer.playground.tierEconomyDesc}
                {selectedTier === 'Standard' && t.buyer.playground.tierStandardDesc}
                {selectedTier === 'Performance' && t.buyer.playground.tierPerformanceDesc}
              </p>
            </div>

            {/* Hyperparameters */}
            <div className="pt-3 border-t border-gray-100 space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-gray-700">{t.buyer.playground.temperature}</span>
                <span className="text-xs font-mono font-bold text-gray-900">{temperature}</span>
              </div>
              <input
                type="range"
                min="0"
                max="2"
                step="0.1"
                value={temperature}
                onChange={(e) => setTemperature(parseFloat(e.target.value))}
                className="w-full accent-gray-900"
              />

              <div className="flex items-center justify-between pt-1">
                <span className="text-xs font-semibold text-gray-700">{t.buyer.playground.maxTokens}</span>
                <span className="text-xs font-mono font-bold text-gray-900">{maxTokens}</span>
              </div>
              <input
                type="range"
                min="256"
                max="8192"
                step="256"
                value={maxTokens}
                onChange={(e) => setMaxTokens(parseInt(e.target.value))}
                className="w-full accent-gray-900"
              />
            </div>
          </BCCard>

          {/* Quick API Snippet Box */}
          <BCCard className="p-4 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5 text-xs font-bold text-gray-900 font-mono">
                <Code2 className="w-3.5 h-3.5" />
                <span>{t.buyer.playground.codeSnippetTitle}</span>
              </div>
              <div className="flex items-center gap-1 bg-gray-100 p-0.5 rounded-lg text-[10px] font-mono">
                {(['curl', 'python', 'node'] as const).map((tab) => (
                  <button
                    key={tab}
                    onClick={() => setActiveCodeTab(tab)}
                    className={`px-2 py-0.5 rounded capitalize cursor-pointer ${
                      activeCodeTab === tab ? 'bg-white font-bold text-gray-900 shadow-xs' : 'text-gray-500'
                    }`}
                  >
                    {tab}
                  </button>
                ))}
              </div>
            </div>

            <pre className="p-3 bg-gray-950 text-gray-100 rounded-xl text-[10px] font-mono overflow-x-auto max-h-48 leading-relaxed">
              {activeCodeTab === 'curl' && getCurlSnippet()}
              {activeCodeTab === 'python' && getPythonSnippet()}
              {activeCodeTab === 'node' && getNodeSnippet()}
            </pre>

            <BCButton
              variant="secondary"
              size="xs"
              onClick={handleCopyCode}
              className="w-full text-[11px]"
            >
              {copiedCode ? <Check className="w-3 h-3 text-emerald-600" /> : <Copy className="w-3 h-3" />}
              <span>{copiedCode ? t.buyer.playground.codeCopied : t.buyer.playground.copyCode}</span>
            </BCButton>
          </BCCard>
        </div>

        {/* Center / Right: Interactive Test & Output */}
        <div className="lg:col-span-8 space-y-4">
          <BCCard className="p-6 space-y-4 flex flex-col justify-between min-h-[500px]">
            {/* System Prompt Input */}
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-gray-700 flex items-center justify-between">
                <span>{t.buyer.playground.systemPrompt}</span>
                <span className="text-[10px] font-mono text-gray-400">Optional</span>
              </label>
              <textarea
                rows={2}
                value={systemPrompt}
                onChange={(e) => setSystemPrompt(e.target.value)}
                placeholder="Specify assistant persona or instructions..."
                className="w-full p-3 bg-gray-50 border border-gray-200/80 rounded-xl text-xs text-gray-900 placeholder:text-gray-400 focus:bg-white focus:outline-none focus:ring-2 focus:ring-gray-900/10 focus:border-gray-900 transition-all resize-none font-mono"
              />
            </div>

            {/* User Prompt Input */}
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-gray-700">{t.buyer.playground.userPrompt}</label>
              <textarea
                rows={4}
                value={userPrompt}
                onChange={(e) => setUserPrompt(e.target.value)}
                placeholder="Enter prompt to execute on BurnCloud..."
                className="w-full p-3 bg-gray-50 border border-gray-200/80 rounded-xl text-xs text-gray-900 placeholder:text-gray-400 focus:bg-white focus:outline-none focus:ring-2 focus:ring-gray-900/10 focus:border-gray-900 transition-all font-sans"
              />
            </div>

            {/* Execution Controls */}
            <div className="flex items-center justify-between pt-2 border-t border-gray-100">
              <div className="flex items-center gap-2 text-xs text-gray-500 font-mono">
                <ShieldCheck className="w-4 h-4 text-emerald-600" />
                <span>{t.buyer.playground.nitroEnclaveVerified}</span>
              </div>

              <div className="flex items-center gap-2">
                <BCButton
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setStreamedResponse('');
                    setExecutionStats(null);
                  }}
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                  <span>{t.buyer.playground.clear}</span>
                </BCButton>

                <BCButton
                  variant="primary"
                  size="md"
                  loading={isLoading}
                  onClick={handleRunInference}
                >
                  <Play className="w-3.5 h-3.5 fill-current" />
                  <span>{isLoading ? t.buyer.playground.running : t.buyer.playground.runInference}</span>
                </BCButton>
              </div>
            </div>

            {/* Output Stream Panel */}
            <div className="mt-4 p-4 rounded-xl bg-gray-50 border border-gray-200/70 space-y-3 flex-1 flex flex-col justify-between">
              <div className="space-y-2">
                <div className="flex items-center justify-between text-xs pb-2 border-b border-gray-200/60">
                  <span className="font-semibold text-gray-700 font-mono uppercase text-[10px]">
                    {t.buyer.playground.responseOutput}
                  </span>
                  {isLoading && (
                    <span className="flex items-center gap-1.5 text-xs text-blue-600 font-mono font-medium">
                      <span className="w-2 h-2 rounded-full bg-blue-600 animate-ping" />
                      {t.buyer.playground.running}
                    </span>
                  )}
                </div>

                <div className="text-xs text-gray-900 font-mono whitespace-pre-wrap leading-relaxed min-h-[140px]">
                  {streamedResponse || (
                    <span className="text-gray-400 italic">
                      Click "{t.buyer.playground.runInference}" above to execute and stream response.
                    </span>
                  )}
                </div>
              </div>

              {/* Execution Telemetry Receipt */}
              {executionStats && (
                <div className="pt-3 border-t border-gray-200/80 grid grid-cols-2 sm:grid-cols-4 gap-2 text-[11px] font-mono text-gray-600">
                  <div className="p-2 bg-white rounded-lg border border-gray-200/60">
                    <span className="text-gray-400 block text-[9px]">{t.buyer.playground.ttft}</span>
                    <span className="font-bold text-gray-900">{executionStats.ttftMs} ms</span>
                  </div>
                  <div className="p-2 bg-white rounded-lg border border-gray-200/60">
                    <span className="text-gray-400 block text-[9px]">{t.buyer.playground.totalTime}</span>
                    <span className="font-bold text-gray-900">{executionStats.totalMs} ms</span>
                  </div>
                  <div className="p-2 bg-white rounded-lg border border-gray-200/60">
                    <span className="text-gray-400 block text-[9px]">{t.buyer.playground.tokensGenerated}</span>
                    <span className="font-bold text-gray-900">
                      {executionStats.promptTokens} / {executionStats.completionTokens}
                    </span>
                  </div>
                  <div className="p-2 bg-white rounded-lg border border-gray-200/60">
                    <span className="text-gray-400 block text-[9px]">{t.buyer.playground.costEstimate}</span>
                    <span className="font-bold text-emerald-700">${executionStats.costUsd}</span>
                  </div>
                </div>
              )}
            </div>
          </BCCard>
        </div>
      </div>
    </div>
  );
}
