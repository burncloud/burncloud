import React, { useState } from 'react';
import { Card, Button, Badge, Input } from '@/components/ui';
import { Terminal, Send, Sparkles, Cpu, Zap, Coins, ArrowRight, CheckCircle2, ShieldCheck, RefreshCw } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';
import { cn } from '@/lib/utils';

const STRATEGIES = [
  { id: 'balanced', name: 'Balanced Optimization', desc: 'Mixes intelligence, cost, and latency dynamically.', icon: Cpu, color: 'text-blue-500', bg: 'bg-blue-50' },
  { id: 'speed', name: 'Extreme Speed', desc: 'Prioritizes lowest latency using fast responsive models.', icon: Zap, color: 'text-amber-500', bg: 'bg-amber-50' },
  { id: 'cost', name: 'Ultra Cost Saver', desc: 'Prioritizes lowest cost using open-source & budget models.', icon: Coins, color: 'text-green-500', bg: 'bg-green-50' },
  { id: 'intelligence', name: 'Max Intelligence', desc: 'Prioritizes complex reasoning with state-of-the-art models.', icon: Sparkles, color: 'text-purple-500', bg: 'bg-purple-50' },
];

export function Playground() {
  const [prompt, setPrompt] = useState('How can I optimize API routing for standard natural language processing queries?');
  const [strategy, setStrategy] = useState('balanced');
  const [isLoading, setIsLoading] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const [routingResult, setRoutingResult] = useState<any | null>(null);

  const simulateLogs = (strategyId: string) => {
    setIsLoading(true);
    setLogs([]);
    setRoutingResult(null);

    const steps = [
      '⚡ [0ms] Request received at BurnCloud Edge Gateway (San Francisco)...',
      '🔍 [35ms] Parsing prompt intent & token count...',
      `🛠️ [85ms] Running policy resolver for strategy "${strategyId.toUpperCase()}"...`,
      '📊 [150ms] Querying active provider health nodes: OpenAI (Healthy), Anthropic (Degraded latency), Google (Healthy), DeepSeek (Healthy)...',
    ];

    if (strategyId === 'balanced') {
      steps.push(
        '🎯 [220ms] Routing Decision: Routed to google/gemini-3.5-flash (Reason: Perfect accuracy balance, active fallback node, optimal cost).',
        '📡 [230ms] Dispatching request payload...'
      );
    } else if (strategyId === 'speed') {
      steps.push(
        '🎯 [140ms] Routing Decision: Routed to google/gemini-3.5-flash (Reason: Lowest latency node 410ms, bypassing degraded Anthropic nodes).',
        '📡 [150ms] Dispatching request payload...'
      );
    } else if (strategyId === 'cost') {
      steps.push(
        '🎯 [180ms] Routing Decision: Routed to deepseek/deepseek-v4 (Reason: Minimum cost threshold matched, 94% saving compared to high-tier nodes).',
        '📡 [190ms] Dispatching request payload...'
      );
    } else {
      steps.push(
        '🎯 [250ms] Routing Decision: Routed to anthropic/claude-3-5-sonnet (Reason: High reasoning capability request, bypassed lightweight models).',
        '📡 [265ms] Dispatching request payload...'
      );
    }

    let currentStep = 0;
    const interval = setInterval(() => {
      if (currentStep < steps.length) {
        setLogs((prev) => [...prev, steps[currentStep]]);
        currentStep++;
      } else {
        clearInterval(interval);
        // Complete the simulation
        setIsLoading(false);
        const result = getSimulationResult(strategyId);
        setRoutingResult(result);
      }
    }, 450);
  };

  const getSimulationResult = (strategyId: string) => {
    switch (strategyId) {
      case 'speed':
        return {
          model: 'Google Gemini 3.5 Flash',
          latency: '410ms',
          cost: '$0.00014',
          tokens: { input: 18, output: 145 },
          status: 'Success',
          cached: false,
          savings: '76%',
          response: 'To optimize API routing for NLP, you should implement edge-caching of prompt hashes and load-balance dynamically based on live provider latencies. BurnCloud handles this out-of-the-box by tracking provider latencies millisecond by millisecond and instantly rerouting around slow or degraded nodes.'
        };
      case 'cost':
        return {
          model: 'DeepSeek V4',
          latency: '610ms',
          cost: '$0.00004',
          tokens: { input: 18, output: 152 },
          status: 'Success',
          cached: false,
          savings: '94%',
          response: 'NLP routing optimization involves analyzing query complexity. Low-complexity tasks can be directed to highly cost-efficient open-source models like DeepSeek-V4 or Llama-3-8B, saving up to 94% on licensing. High-complexity queries can be gracefully routed to higher-tier reasoning engines only when needed.'
        };
      case 'intelligence':
        return {
          model: 'Anthropic Claude 3.5 Sonnet',
          latency: '1,420ms',
          cost: '$0.00180',
          tokens: { input: 18, output: 168 },
          status: 'Success',
          cached: false,
          savings: '12%',
          response: 'Comprehensive natural language processing optimization requires robust multi-tiered router gateways. High-reasoning agents analyze prompt semantic characteristics, evaluate if safety guardrails are needed, apply semantic routing caches, and direct tasks requiring code generation or complex math to premium reasoning clusters.'
        };
      case 'balanced':
      default:
        return {
          model: 'Google Gemini 3.5 Flash',
          latency: '425ms',
          cost: '$0.00014',
          tokens: { input: 18, output: 158 },
          status: 'Success',
          cached: true,
          savings: '82%',
          response: 'Optimizing NLP API routing relies on three core tenets:\n1. Edge Caching: Instant, free answers for repeated prompts.\n2. Semantic Classification: Directing easy greetings or classifications to low-cost models, reserving complex tasks for heavy reasoning engines.\n3. Failover Orchestration: Instantly sliding traffic away from failing endpoints with zero downtime.'
        };
    }
  };

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out">
      {/* Intro Header */}
      <div>
        <h2 className="text-[26px] font-display font-semibold text-gray-900 tracking-tight">AI Router Playground</h2>
        <p className="text-gray-500 mt-1.5 text-[14px]">Experience the speed, intelligence, and savings of the BurnCloud Edge Routing Layer in real time.</p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-8">
        {/* Left Side: Inputs */}
        <div className="lg:col-span-5 space-y-6">
          <Card className="p-6 space-y-6">
            {/* Strategy selection */}
            <div className="space-y-3">
              <label className="text-[13px] font-semibold text-gray-700 tracking-wide uppercase">Select Routing Strategy</label>
              <div className="grid grid-cols-1 gap-2.5">
                {STRATEGIES.map((strat) => {
                  const Icon = strat.icon;
                  const isSelected = strategy === strat.id;
                  return (
                    <button
                      key={strat.id}
                      onClick={() => !isLoading && setStrategy(strat.id)}
                      disabled={isLoading}
                      className={cn(
                        "flex items-start gap-4 p-3.5 rounded-xl border text-left transition-all",
                        isSelected 
                          ? "border-orange-500/50 bg-orange-50/20 shadow-sm ring-1 ring-orange-500/20" 
                          : "border-gray-200/80 hover:bg-gray-50/50 hover:border-gray-300"
                      )}
                    >
                      <div className={cn("p-2 rounded-lg mt-0.5", isSelected ? "bg-orange-500/10 text-orange-600" : "bg-gray-100 text-gray-500")}>
                        <Icon className="w-4 h-4" />
                      </div>
                      <div>
                        <div className="text-[13.5px] font-semibold text-gray-900 flex items-center gap-1.5">
                          {strat.name}
                          {isSelected && <span className="w-1.5 h-1.5 bg-orange-500 rounded-full"></span>}
                        </div>
                        <p className="text-xs text-gray-500 mt-1 leading-normal">{strat.desc}</p>
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>

            {/* Prompt input */}
            <div className="space-y-3">
              <label className="text-[13px] font-semibold text-gray-700 tracking-wide uppercase">Your Prompt</label>
              <div className="relative">
                <textarea
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  disabled={isLoading}
                  rows={4}
                  className="w-full bg-gray-50/50 border border-gray-200/80 rounded-xl p-3.5 text-[13px] focus:bg-white focus:ring-4 focus:ring-orange-500/5 focus:border-orange-500/30 transition-all placeholder:text-gray-400 focus:outline-none leading-relaxed"
                  placeholder="Type a query to see how BurnCloud routes it..."
                />
              </div>
            </div>

            {/* Run Button */}
            <Button
              onClick={() => simulateLogs(strategy)}
              disabled={isLoading || !prompt.trim()}
              className="w-full h-11 bg-gradient-to-b from-orange-500 to-orange-600 hover:from-orange-600 hover:to-orange-700 text-white font-medium rounded-xl shadow-[0_4px_12px_rgba(233,85,19,0.2)] border-0 flex items-center justify-center gap-2 cursor-pointer transition-all disabled:opacity-50"
            >
              {isLoading ? (
                <>
                  <RefreshCw className="w-4 h-4 animate-spin" />
                  Routing Request...
                </>
              ) : (
                <>
                  <Send className="w-4 h-4" />
                  Run Routing Test
                </>
              )}
            </Button>
          </Card>
        </div>

        {/* Right Side: Visual routing & terminal logs */}
        <div className="lg:col-span-7 space-y-6">
          <Card className="p-6 bg-gray-950 text-gray-100 font-mono text-[12.5px] min-h-[460px] flex flex-col justify-between border-gray-900 shadow-2xl relative overflow-hidden">
            {/* Ambient terminal background glow */}
            <div className="absolute top-0 right-0 w-[200px] h-[200px] bg-orange-600/5 rounded-full blur-[100px] pointer-events-none" />

            {/* Top terminal bar */}
            <div className="flex items-center justify-between pb-4 border-b border-gray-900/80 mb-4">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-red-500/80"></div>
                <div className="w-3 h-3 rounded-full bg-yellow-500/80"></div>
                <div className="w-3 h-3 rounded-full bg-green-500/80"></div>
                <span className="text-gray-500 ml-2 font-sans text-xs">burncloud-edge-router.sh</span>
              </div>
              <Badge className="bg-gray-900 border-gray-800 text-orange-400 text-[10px]">v1.4.2</Badge>
            </div>

            {/* Logs Window */}
            <div className="flex-1 space-y-3 overflow-y-auto mb-4 min-h-[220px]">
              {logs.length === 0 && !isLoading && (
                <div className="h-full flex flex-col items-center justify-center text-center text-gray-500 py-12 space-y-3 font-sans">
                  <Terminal className="w-8 h-8 text-gray-700 animate-pulse" />
                  <p>Ready to trace routing execution. Click 'Run Routing Test' to watch the magic happen.</p>
                </div>
              )}

              {logs.map((log, index) => (
                <motion.div
                  key={index}
                  initial={{ opacity: 0, x: -10 }}
                  animate={{ opacity: 1, x: 0 }}
                  className={cn(
                    "leading-relaxed",
                    log.includes('Routed to') ? 'text-green-400 font-semibold' : 'text-gray-300'
                  )}
                >
                  {log}
                </motion.div>
              ))}

              {isLoading && (
                <div className="flex items-center gap-2 text-orange-400 font-sans mt-2">
                  <span className="w-2 h-2 rounded-full bg-orange-500 animate-ping"></span>
                  <span className="text-xs">Processing latency metrics...</span>
                </div>
              )}
            </div>

            {/* Decisive Router Output */}
            <AnimatePresence>
              {routingResult && (
                <motion.div
                  initial={{ opacity: 0, y: 15 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: 10 }}
                  className="pt-4 border-t border-gray-900 bg-gray-950 z-10"
                >
                  {/* Visual Node Selected */}
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-4 font-sans text-xs text-gray-400">
                    <div className="bg-gray-900/80 p-2.5 rounded-lg border border-gray-900">
                      <span className="block text-[10px] text-gray-500 uppercase tracking-wider">Model Used</span>
                      <span className="text-[13px] font-semibold text-white mt-1 block truncate">{routingResult.model}</span>
                    </div>
                    <div className="bg-gray-900/80 p-2.5 rounded-lg border border-gray-900">
                      <span className="block text-[10px] text-gray-500 uppercase tracking-wider">Latency</span>
                      <span className="text-[13px] font-semibold text-green-400 mt-1 block">{routingResult.latency}</span>
                    </div>
                    <div className="bg-gray-900/80 p-2.5 rounded-lg border border-gray-900">
                      <span className="block text-[10px] text-gray-500 uppercase tracking-wider">Estimated Cost</span>
                      <span className="text-[13px] font-semibold text-white mt-1 block">{routingResult.cost}</span>
                    </div>
                    <div className="bg-gray-900/80 p-2.5 rounded-lg border border-gray-900">
                      <span className="block text-[10px] text-gray-500 uppercase tracking-wider">Cost Saved</span>
                      <span className="text-[13px] font-semibold text-orange-400 mt-1 block flex items-center gap-1">
                        <Coins className="w-3.5 h-3.5" />
                        {routingResult.savings}
                      </span>
                    </div>
                  </div>

                  {/* Text Response */}
                  <div className="bg-gray-900/50 p-4 rounded-xl border border-gray-900 text-gray-300 font-sans leading-relaxed text-[13px]">
                    <div className="flex items-center gap-2 mb-2 text-xs font-semibold text-gray-400">
                      <ShieldCheck className="w-4 h-4 text-green-500" />
                      SECURE MODEL OUTPUT:
                      {routingResult.cached && <Badge className="bg-green-500/10 border-green-500/20 text-green-400 text-[10px] ml-auto py-0 px-2">Prompt Cached Hit</Badge>}
                    </div>
                    <p className="whitespace-pre-line">{routingResult.response}</p>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </Card>
        </div>
      </div>
    </div>
  );
}
