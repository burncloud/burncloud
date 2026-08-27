export type Environment = 'Production' | 'Staging' | 'Development';

export interface Route {
  id: string;
  name: string;
  environment: Environment;
  primaryModel: string;
  fallbackChain: string[];
  traffic: number;
  successRate: number;
  avgLatency: number;
  costPer1M: number;
  status: 'Active' | 'Testing' | 'Paused';
}

export interface Model {
  id: string;
  name: string;
  provider: string;
  tags: string[];
  contextWindow: string;
  inputCost: number;
  outputCost: number;
  latency: number;
  reliability: number;
  quality: number;
}

export interface Provider {
  id: string;
  name: string;
  status: 'Connected' | 'Degraded' | 'Outage' | 'Disconnected';
  keyHealth: 'Valid' | 'Invalid';
  rateLimitUsage: number;
  monthlySpend: number;
  lastIncident: string;
  enabledRoutes: number;
}

export interface Log {
  id: string;
  timestamp: string;
  requestId: string;
  customer: string;
  route: string;
  model: string;
  provider: string;
  status: 'Success' | 'Fallback' | 'Timeout' | 'Error';
  latency: number;
  tokens: number;
  cost: number;
  fallbackUsed: boolean;
  fallbackTo?: string;
}

export const MOCK_ROUTES: Route[] = [
  { id: '1', name: 'production-chat-default', environment: 'Production', primaryModel: 'claude-fable-5', fallbackChain: ['gpt-5.5', 'gemini-3.5-flash', 'DeepSeek-V4'], traffic: 42, successRate: 99.99, avgLatency: 580, costPer1M: 3.50, status: 'Active' },
  { id: '2', name: 'cost-optimized-general', environment: 'Production', primaryModel: 'DeepSeek-V4', fallbackChain: ['Qwen/Qwen3.6-35B-A3B', 'gemini-3.5-flash'], traffic: 28, successRate: 99.94, avgLatency: 450, costPer1M: 0.21, status: 'Active' },
  { id: '3', name: 'coding-agent-premium', environment: 'Production', primaryModel: 'claude-fable-5', fallbackChain: ['gpt-5.5', 'kimi-k2.7-code'], traffic: 18, successRate: 99.97, avgLatency: 820, costPer1M: 4.80, status: 'Active' },
  { id: '4', name: 'chinese-long-context', environment: 'Production', primaryModel: 'GLM-5.2', fallbackChain: ['Qwen/Qwen3.6-35B-A3B', 'Seed2.0 Pro'], traffic: 9, successRate: 99.91, avgLatency: 680, costPer1M: 0.60, status: 'Active' },
  { id: '5', name: 'experimental-reasoning', environment: 'Staging', primaryModel: 'DeepSeek-V4', fallbackChain: ['gpt-5.5', 'claude-fable-5'], traffic: 3, successRate: 99.70, avgLatency: 1100, costPer1M: 1.20, status: 'Testing' }
];

export const MOCK_MODELS: Model[] = [
  { id: '1', name: 'claude-fable-5', provider: 'Anthropic', tags: ['Agentic', 'Coding', 'Complex Tasks'], contextWindow: '200K tokens', inputCost: 3.00, outputCost: 15.00, latency: 850, reliability: 99.98, quality: 99 },
  { id: '2', name: 'gpt-5.5', provider: 'OpenAI', tags: ['Reasoning', 'Coding', 'Multimodal'], contextWindow: '200K tokens', inputCost: 2.00, outputCost: 8.00, latency: 620, reliability: 99.99, quality: 99 },
  { id: '3', name: 'gemini-3.5-flash', provider: 'Google', tags: ['Multimodal', 'Coding', 'Agentic'], contextWindow: '2M tokens', inputCost: 0.10, outputCost: 0.40, latency: 410, reliability: 99.97, quality: 95 },
  { id: '4', name: 'grok-4.5', provider: 'xAI', tags: ['Real-time Search', 'Coding', 'Tool Calling'], contextWindow: '128K tokens', inputCost: 2.00, outputCost: 10.00, latency: 510, reliability: 99.92, quality: 96 },
  { id: '5', name: 'kimi-k2.7-code', provider: 'Kimi', tags: ['Coding', 'High Speed', 'Agentic'], contextWindow: '200K tokens', inputCost: 0.20, outputCost: 0.80, latency: 480, reliability: 99.94, quality: 94 },
  { id: '6', name: 'Seed2.0 Pro', provider: 'Doubao', tags: ['Chinese Context', 'Multimodal', 'Reasoning'], contextWindow: '128K tokens', inputCost: 0.15, outputCost: 0.60, latency: 380, reliability: 99.95, quality: 93 },
  { id: '7', name: 'Llama-4-Maverick', provider: 'Meta', tags: ['Open Weights', 'Multimodal', 'Local Setup'], contextWindow: '128K tokens', inputCost: 0.00, outputCost: 0.00, latency: 220, reliability: 99.96, quality: 91 },
  { id: '8', name: 'Qwen/Qwen3.6-35B-A3B', provider: 'Alibaba Cloud', tags: ['Open Weights', 'Chinese', 'Coding'], contextWindow: '128K tokens', inputCost: 0.10, outputCost: 0.30, latency: 440, reliability: 99.95, quality: 93 },
  { id: '9', name: 'DeepSeek-V4', provider: 'DeepSeek', tags: ['Low Cost', 'Reasoning', 'Open Weights'], contextWindow: '128K tokens', inputCost: 0.14, outputCost: 0.28, latency: 610, reliability: 99.94, quality: 97 },
  { id: '10', name: 'GLM-5.2', provider: 'GLM', tags: ['Long Context', 'Agentic', 'Chinese'], contextWindow: '1M tokens', inputCost: 0.50, outputCost: 1.50, latency: 720, reliability: 99.92, quality: 95 }
];

export const MOCK_LOGS: Log[] = [
  { id: '1', timestamp: '10:42:18', requestId: 'req_8f29a1', customer: 'ETR Global', route: 'production-chat-default', model: 'claude-fable-5', provider: 'Anthropic', status: 'Success', latency: 742, tokens: 4820, cost: 0.038, fallbackUsed: false },
  { id: '2', timestamp: '10:42:11', requestId: 'req_8f29a0', customer: 'NovaDesk', route: 'cost-optimized-general', model: 'DeepSeek-V4', provider: 'DeepSeek', status: 'Fallback', latency: 4120, tokens: 2940, cost: 0.006, fallbackUsed: true, fallbackTo: 'Qwen/Qwen3.6-35B-A3B' },
  { id: '3', timestamp: '10:41:59', requestId: 'req_8f299f', customer: 'Internal', route: 'coding-agent-premium', model: 'claude-fable-5', provider: 'Anthropic', status: 'Timeout', latency: 10000, tokens: 7220, cost: 0.108, fallbackUsed: true, fallbackTo: 'gpt-5.5' }
];
