export interface ModelItem {
  id: string;
  name: string;
  family: string;
  tagline: string;
  description: string;
  category: 'General LLM' | 'Reasoning & Math' | 'Coding' | 'Multimodal' | 'Low Latency';
  inputPrice1M: number;
  outputPrice1M: number;
  officialInputPrice1M?: number;
  officialOutputPrice1M?: number;
  contextWindow: string;
  availability: number; // e.g. 99.98%
  status: 'Healthy' | 'Degraded' | 'At Risk' | 'Capacity Constrained';
  supportedTiers: ('Economy' | 'Standard' | 'Performance')[];
  defaultTier: 'Economy' | 'Standard' | 'Performance';
  p95LatencyMs: number;
  throughputTokensPerSec: number;
  capabilities: string[];
  benchmarks: { name: string; score: string }[];
  recommendedFor: string;
}

export const WORKBENCH_MODELS: ModelItem[] = [
  {
    id: 'deepseek-v3',
    name: 'DeepSeek V3 (671B MoE)',
    family: 'DeepSeek',
    tagline: 'Leading open-weights frontier model with top-tier efficiency.',
    description: 'High-throughput 671B Mixture-of-Experts architecture delivering GPT-4o parity at 1/10th the inference cost. Optimized for agentic execution and general intelligence.',
    category: 'General LLM',
    inputPrice1M: 0.14,
    outputPrice1M: 0.28,
    officialInputPrice1M: 0.14,
    officialOutputPrice1M: 0.28,
    contextWindow: '128K tokens',
    availability: 99.99,
    status: 'Healthy',
    supportedTiers: ['Economy', 'Standard', 'Performance'],
    defaultTier: 'Standard',
    p95LatencyMs: 380,
    throughputTokensPerSec: 85,
    capabilities: ['Tool Calling', 'Structured JSON Output', 'System Prompts', 'FIM Completion'],
    benchmarks: [
      { name: 'MMLU-Pro', score: '75.9%' },
      { name: 'HumanEval', score: '82.6%' },
      { name: 'MATH-500', score: '90.2%' }
    ],
    recommendedFor: 'Enterprise production agents, high-volume classification, and conversational backends.'
  },
  {
    id: 'deepseek-r1',
    name: 'DeepSeek R1 Reasoning',
    family: 'DeepSeek',
    tagline: 'Chain-of-Thought deep reasoning for complex coding, math, and architecture.',
    description: 'Incentivized reinforcement learning model exhibiting self-verification and deep multi-step deduction for hard technical questions and algorithmic design.',
    category: 'Reasoning & Math',
    inputPrice1M: 0.55,
    outputPrice1M: 2.19,
    officialInputPrice1M: 0.55,
    officialOutputPrice1M: 2.19,
    contextWindow: '128K tokens',
    availability: 99.95,
    status: 'Healthy',
    supportedTiers: ['Standard', 'Performance'],
    defaultTier: 'Standard',
    p95LatencyMs: 620,
    throughputTokensPerSec: 55,
    capabilities: ['Extended Thinking', 'Verification Loops', 'LaTeX Math', 'Algorithm Synthesis'],
    benchmarks: [
      { name: 'AIME 2024', score: '79.8%' },
      { name: 'MATH-500', score: '97.3%' },
      { name: 'Codeforces', score: '96.3th percentile' }
    ],
    recommendedFor: 'Complex algorithmic challenges, financial modeling, and self-correcting agents.'
  },
  {
    id: 'qwen-2.5-72b-instruct',
    name: 'Qwen 2.5 72B Instruct',
    family: 'Qwen',
    tagline: 'Versatile multilingual powerhouse with exceptional instruction following.',
    description: 'Superb multilingual comprehension with state-of-the-art coding and math benchmarks. Robust reasoning with low TTFT latency.',
    category: 'Coding',
    inputPrice1M: 0.35,
    outputPrice1M: 0.70,
    officialInputPrice1M: 0.40,
    officialOutputPrice1M: 0.80,
    contextWindow: '128K tokens',
    availability: 99.98,
    status: 'Healthy',
    supportedTiers: ['Economy', 'Standard', 'Performance'],
    defaultTier: 'Standard',
    p95LatencyMs: 410,
    throughputTokensPerSec: 72,
    capabilities: ['29+ Languages', 'Long Context', 'JSON Extraction', 'Role Playing'],
    benchmarks: [
      { name: 'MMLU', score: '86.1%' },
      { name: 'LiveCodeBench', score: '42.8%' },
      { name: 'Arena-Hard', score: '81.2' }
    ],
    recommendedFor: 'Global multilingual applications, coding copilots, and structured data pipelines.'
  },
  {
    id: 'claude-3-5-sonnet',
    name: 'Claude 3.5 Sonnet Pass-Through',
    family: 'Anthropic',
    tagline: 'Industry benchmark for code generation and nuanced instruction following.',
    description: 'Direct attestation pass-through with hardware receipt cryptographic verification and zero telemetry retention.',
    category: 'Coding',
    inputPrice1M: 3.00,
    outputPrice1M: 15.00,
    officialInputPrice1M: 3.00,
    officialOutputPrice1M: 15.00,
    contextWindow: '200K tokens',
    availability: 99.99,
    status: 'Healthy',
    supportedTiers: ['Standard', 'Performance'],
    defaultTier: 'Performance',
    p95LatencyMs: 540,
    throughputTokensPerSec: 68,
    capabilities: ['Artifact Rendering', 'Vision Parsing', 'Complex Refactoring', 'Tool Calling'],
    benchmarks: [
      { name: 'SWE-bench Verified', score: '49.0%' },
      { name: 'TAU-bench', score: '69.2%' }
    ],
    recommendedFor: 'High-autonomy coding agents, UI synthesis, and mission-critical workflows.'
  },
  {
    id: 'llama-3.3-70b-instruct',
    name: 'Llama 3.3 70B Instruct',
    family: 'Meta',
    tagline: 'High-speed open standard for reliable enterprise generation.',
    description: 'Meta flagship open weights model fine-tuned for dense instruction adherence, low-jitter throughput, and high concurrency.',
    category: 'Low Latency',
    inputPrice1M: 0.60,
    outputPrice1M: 0.60,
    officialInputPrice1M: 0.65,
    officialOutputPrice1M: 0.65,
    contextWindow: '128K tokens',
    availability: 99.97,
    status: 'Healthy',
    supportedTiers: ['Economy', 'Standard'],
    defaultTier: 'Economy',
    p95LatencyMs: 290,
    throughputTokensPerSec: 96,
    capabilities: ['Fast Completion', 'Function Calling', 'RAG Embed Support'],
    benchmarks: [
      { name: 'MMLU', score: '88.6%' },
      { name: 'GSM8K', score: '95.0%' }
    ],
    recommendedFor: 'Cost-sensitive real-time chat, summarization, and batch RAG pipelines.'
  },
  {
    id: 'glm-4-plus',
    name: 'GLM-4 Plus',
    family: 'Zhipu',
    tagline: 'Massive context comprehension with Chinese language mastery.',
    description: 'State-of-the-art Chinese comprehension and bilingual reasoning with 1M tokens extended context handling capabilities.',
    category: 'General LLM',
    inputPrice1M: 0.70,
    outputPrice1M: 1.40,
    contextWindow: '1M tokens',
    availability: 99.91,
    status: 'Degraded',
    supportedTiers: ['Standard', 'Performance'],
    defaultTier: 'Standard',
    p95LatencyMs: 780,
    throughputTokensPerSec: 48,
    capabilities: ['1M Long Document QA', 'Bilingual Translation', 'Workflow Logic'],
    benchmarks: [
      { name: 'LongBench-Chat', score: '89.4%' }
    ],
    recommendedFor: 'Full-book processing, legal contract analysis, and bilingual knowledge search.'
  }
];

// Buyer Mock State
export interface BuyerApiKey {
  id: string;
  name: string;
  keyPrefix: string;
  maskedKey: string;
  created: string;
  lastUsed: string;
  tier: 'All Tiers' | 'Standard & Economy' | 'Performance Only';
  rateLimitRpm: number;
  monthlySpendCap: number;
  spendThisMonth: number;
  status: 'Active' | 'Revoked';
}

export const MOCK_BUYER_KEYS: BuyerApiKey[] = [
  {
    id: 'key-prod-01',
    name: 'Production Kubernetes Cluster (US-West)',
    keyPrefix: 'demo-bc-prod',
    maskedKey: 'demo-bc-prod••••••••••••••••3d8f',
    created: '2026-06-12',
    lastUsed: 'Just now',
    tier: 'All Tiers',
    rateLimitRpm: 1200,
    monthlySpendCap: 2500,
    spendThisMonth: 842.10,
    status: 'Active'
  },
  {
    id: 'key-dev-agent',
    name: 'Dev Agent Sandbox (CI/CD)',
    keyPrefix: 'demo-bc-agent',
    maskedKey: 'demo-bc-agent••••••••••••••••89c2',
    created: '2026-07-04',
    lastUsed: '14 mins ago',
    tier: 'Standard & Economy',
    rateLimitRpm: 300,
    monthlySpendCap: 500,
    spendThisMonth: 128.40,
    status: 'Active'
  },
  {
    id: 'key-eval-bench',
    name: 'Model Benchmark & Automated Evaluation',
    keyPrefix: 'demo-bc-analytics',
    maskedKey: 'demo-bc-analytics••••••••••••••••1120',
    created: '2026-08-01',
    lastUsed: '2 days ago',
    tier: 'All Tiers',
    rateLimitRpm: 600,
    monthlySpendCap: 300,
    spendThisMonth: 42.15,
    status: 'Active'
  }
];

// Supplier Mock State
export interface SupplierGpuNode {
  id: string;
  name: string;
  cluster: string;
  region: string;
  gpuCount: number;
  gpuType: string;
  vramTotalGb: number;
  pcieBandwidth: string;
  temperatureC: number;
  status: 'Online' | 'Offline' | 'Draining' | 'Degraded';
  utilization: number;
  assignedModel: string;
  uptimeHours: number;
  earningsToday: number;
}

export const MOCK_SUPPLIER_NODES: SupplierGpuNode[] = [
  {
    id: 'node-us-sjc-01',
    name: 'SJC-Pod-01-Rack4',
    cluster: 'Silicon-Bay-A',
    region: 'us-west-sjc',
    gpuCount: 8,
    gpuType: 'NVIDIA H100 SXM5 80GB',
    vramTotalGb: 640,
    pcieBandwidth: 'NVLink 900 GB/s',
    temperatureC: 58,
    status: 'Online',
    utilization: 91.4,
    assignedModel: 'DeepSeek V3 (Standard Tier)',
    uptimeHours: 742,
    earningsToday: 184.20
  },
  {
    id: 'node-us-sjc-02',
    name: 'SJC-Pod-01-Rack5',
    cluster: 'Silicon-Bay-A',
    region: 'us-west-sjc',
    gpuCount: 8,
    gpuType: 'NVIDIA H100 SXM5 80GB',
    vramTotalGb: 640,
    pcieBandwidth: 'NVLink 900 GB/s',
    temperatureC: 61,
    status: 'Online',
    utilization: 88.6,
    assignedModel: 'DeepSeek R1 (Performance Tier)',
    uptimeHours: 512,
    earningsToday: 178.60
  },
  {
    id: 'node-eu-fra-01',
    name: 'FRA-DC2-Compute-08',
    cluster: 'Frankfurt-EuroNode',
    region: 'eu-central-fra',
    gpuCount: 8,
    gpuType: 'NVIDIA A100-SXM4 80GB',
    vramTotalGb: 640,
    pcieBandwidth: 'NVLink 600 GB/s',
    temperatureC: 64,
    status: 'Online',
    utilization: 74.2,
    assignedModel: 'Qwen 2.5 72B (Standard Tier)',
    uptimeHours: 1240,
    earningsToday: 79.40
  },
  {
    id: 'node-ap-hkg-03',
    name: 'HKG-Edge-RTX-Pool',
    cluster: 'HKG-Community-01',
    region: 'ap-east-hkg',
    gpuCount: 4,
    gpuType: 'NVIDIA RTX 4090 24GB',
    vramTotalGb: 96,
    pcieBandwidth: 'PCIe 4.0 x16',
    temperatureC: 72,
    status: 'Degraded',
    utilization: 42.0,
    assignedModel: 'Llama 3.3 70B Quantized (Economy)',
    uptimeHours: 88,
    earningsToday: 18.20
  }
];

// Admin Platform Mock State
export interface AdminAutopilotLog {
  id: string;
  time: string;
  level: 'Action' | 'Warning' | 'Optimization' | 'Failover';
  category: 'Capacity' | 'Failover' | 'Economics' | 'Traffic';
  title: string;
  description: string;
  impact: string;
  actionTaken: string;
}

export const MOCK_AUTOPILOT_LOGS: AdminAutopilotLog[] = [
  {
    id: 'auto-101',
    time: '2 mins ago',
    level: 'Action',
    category: 'Capacity',
    title: 'Autopilot added 16x H100 capacity to DeepSeek V3',
    description: 'Demand surge detected in US-West (+34% concurrency). Autopilot provisioned idle supplier nodes without operator intervention.',
    impact: 'p95 latency stabilized from 620ms to 380ms.',
    actionTaken: 'Auto-scaled 2 standby worker nodes.'
  },
  {
    id: 'auto-102',
    time: '18 mins ago',
    level: 'Failover',
    category: 'Failover',
    title: 'Isolated unstable Node ap-hkg-03 due to PCIe thermal warning',
    description: 'Node temperature exceeded 72C. Autopilot drained in-flight requests and rerouted traffic seamlessly to Shenzhen cluster.',
    impact: '0 customer requests dropped; 100% SLA preserved.',
    actionTaken: 'Marked node Degraded and alerted supplier.'
  },
  {
    id: 'auto-103',
    time: '1 hour ago',
    level: 'Optimization',
    category: 'Economics',
    title: 'Optimized Batch Inference routing for Economy tier',
    description: 'Rerouted 4.2M non-time-critical tokens to off-peak compute nodes.',
    impact: 'Platform gross margin increased by +2.4%.',
    actionTaken: 'Updated dynamic batching parameters.'
  }
];
